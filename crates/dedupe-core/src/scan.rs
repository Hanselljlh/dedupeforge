use crate::fs_walk::{collect_files, FileEntry};
use crate::hash::{hash_file, hash_file_prefix, HashAlgorithm};
use crate::verify::files_equal;
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanConfig {
    pub paths: Vec<PathBuf>,
    pub protected_roots: Vec<PathBuf>,
    pub algorithm: HashAlgorithm,
    pub partial_bytes: u64,
    pub min_size: u64,
    pub ignore_hidden: bool,
    pub byte_verify: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DuplicateItem {
    pub path: PathBuf,
    pub size: u64,
    pub modified_unix: Option<i64>,
    pub is_protected: bool,
    pub suggested_keep: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub size: u64,
    pub algorithm: String,
    pub hash: String,
    pub reason: String,
    pub items: Vec<DuplicateItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanReport {
    pub scanned_files: usize,
    pub candidate_size_groups: usize,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub errors: Vec<String>,
}

pub fn scan_exact(config: &ScanConfig) -> Result<ScanReport> {
    let files = collect_files(&config.paths, &config.protected_roots, config.ignore_hidden)?
        .into_iter()
        .filter(|f| f.size >= config.min_size)
        .collect::<Vec<_>>();

    let scanned_files = files.len();
    let by_size = group_by_size(files);
    let candidate_size_groups = by_size.values().filter(|g| g.len() > 1).count();

    let mut errors = Vec::new();
    let size_candidate_files: Vec<FileEntry> = by_size.into_values().flatten().collect();
    let partial_candidates = hash_files(size_candidate_files, config.algorithm, config.partial_bytes, true, &mut errors);
    let partial_candidate_files: Vec<FileEntry> = partial_candidates.into_values().flatten().collect();
    let full_candidates = hash_files(partial_candidate_files, config.algorithm, 0, false, &mut errors);

    let mut duplicate_groups = Vec::new();

    for ((_size, full_hash), mut group) in full_candidates {
        if group.len() < 2 { continue; }
        group.sort_by_key(item_sort_key);

        if config.byte_verify {
            for verified in split_by_byte_equality(&group, &mut errors) {
                if verified.len() > 1 {
                    duplicate_groups.push(make_group(verified, config.algorithm, full_hash.clone(), true));
                }
            }
        } else {
            duplicate_groups.push(make_group(group, config.algorithm, full_hash, false));
        }
    }

    duplicate_groups.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.items[0].path.cmp(&b.items[0].path)));

    Ok(ScanReport {
        scanned_files,
        candidate_size_groups,
        duplicate_groups,
        errors,
    })
}

fn group_by_size(files: Vec<FileEntry>) -> HashMap<u64, Vec<FileEntry>> {
    let mut map: HashMap<u64, Vec<FileEntry>> = HashMap::new();
    for file in files {
        map.entry(file.size).or_default().push(file);
    }
    map.retain(|_, group| group.len() > 1);
    map
}

fn hash_files(
    files: Vec<FileEntry>,
    algorithm: HashAlgorithm,
    partial_bytes: u64,
    partial: bool,
    errors: &mut Vec<String>,
) -> HashMap<(u64, String), Vec<FileEntry>> {
    let hashed: Vec<(FileEntry, Option<String>)> = files
        .into_par_iter()
        .map(|file| {
            let hash_result = if partial {
                hash_file_prefix(&file.path, algorithm, partial_bytes)
            } else {
                hash_file(&file.path, algorithm)
            };
            match hash_result {
                Ok(hash) => (file, Some(hash)),
                Err(e) => (file, Some(format!("ERROR::{e}"))),
            }
        })
        .collect();

    let mut map: HashMap<(u64, String), Vec<FileEntry>> = HashMap::new();

    for (file, maybe_hash) in hashed {
        match maybe_hash {
            Some(hash) if hash.starts_with("ERROR::") => errors.push(format!("{}: {}", file.path.display(), hash.trim_start_matches("ERROR::"))),
            Some(hash) => { map.entry((file.size, hash)).or_default().push(file); }
            None => {}
        }
    }

    map.retain(|_, group| group.len() > 1);
    map
}

fn split_by_byte_equality(group: &[FileEntry], errors: &mut Vec<String>) -> Vec<Vec<FileEntry>> {
    let mut verified_groups: Vec<Vec<FileEntry>> = Vec::new();

    'outer: for item in group.iter().cloned() {
        for existing_group in verified_groups.iter_mut() {
            let representative = &existing_group[0];
            match files_equal(&representative.path, &item.path) {
                Ok(true) => {
                    existing_group.push(item);
                    continue 'outer;
                }
                Ok(false) => {}
                Err(e) => errors.push(format!("byte verify failed for {}: {e}", item.path.display())),
            }
        }
        verified_groups.push(vec![item]);
    }

    verified_groups
}

fn make_group(mut files: Vec<FileEntry>, algorithm: HashAlgorithm, hash: String, byte_verified: bool) -> DuplicateGroup {
    files.sort_by_key(item_sort_key);
    let keep_index = choose_keep_index(&files);
    let items = files.into_iter().enumerate().map(|(idx, f)| DuplicateItem {
        path: f.path,
        size: f.size,
        modified_unix: f.modified_unix,
        is_protected: f.is_protected,
        suggested_keep: idx == keep_index,
    }).collect();

    DuplicateGroup {
        size: items_first_size(&items),
        algorithm: algorithm.label().to_string(),
        hash,
        reason: if byte_verified {
            "same size + same full hash + byte-by-byte verified".to_string()
        } else {
            "same size + same full hash".to_string()
        },
        items,
    }
}

fn items_first_size(items: &[DuplicateItem]) -> u64 {
    items.first().map(|i| i.size).unwrap_or_default()
}

fn choose_keep_index(files: &[FileEntry]) -> usize {
    files.iter().position(|f| f.is_protected).unwrap_or(0)
}

fn item_sort_key(f: &FileEntry) -> (bool, Option<i64>, usize, String) {
    (
        !f.is_protected,
        f.modified_unix,
        f.path.components().count(),
        f.path.to_string_lossy().to_lowercase(),
    )
}


#[cfg(test)]
mod tests {
        use super::*;
        use std::fs;
        use tempfile::tempdir;

    fn base_config(paths: Vec<PathBuf>, protected: Vec<PathBuf>) -> ScanConfig {
                ScanConfig {
                                paths,
                                protected_roots: protected,
                                algorithm: HashAlgorithm::Blake3,
                                partial_bytes: 4096,
                                min_size: 1,
                                ignore_hidden: false,
                                byte_verify: false,
                }
    }

    fn write(dir: &std::path::Path, name: &str, content: &[u8]) {
                fs::write(dir.join(name), content).unwrap();
    }

    #[test]
        fn two_identical_files_form_one_group() {
                    let dir = tempdir().unwrap();
                    write(dir.path(), "a.txt", b"duplicate content");
                    write(dir.path(), "b.txt", b"duplicate content");
                    write(dir.path(), "c.txt", b"unique content here");
                    let report = scan_exact(&base_config(vec![dir.path().to_path_buf()], vec![])).unwrap();
                    assert_eq!(report.duplicate_groups.len(), 1);
                    assert_eq!(report.duplicate_groups[0].items.len(), 2);
        }

    #[test]
        fn no_duplicates_produces_empty_groups() {
                    let dir = tempdir().unwrap();
                    write(dir.path(), "a.txt", b"aaa");
                    write(dir.path(), "b.txt", b"bbb");
                    write(dir.path(), "c.txt", b"ccc");
                    let report = scan_exact(&base_config(vec![dir.path().to_path_buf()], vec![])).unwrap();
                    assert_eq!(report.duplicate_groups.len(), 0);
                    assert_eq!(report.scanned_files, 3);
        }

    #[test]
        fn files_below_min_size_are_excluded() {
                    let dir = tempdir().unwrap();
                    write(dir.path(), "a.txt", b"x");
                    write(dir.path(), "b.txt", b"x");
                    let mut config = base_config(vec![dir.path().to_path_buf()], vec![]);
                    config.min_size = 100;
                    let report = scan_exact(&config).unwrap();
                    assert_eq!(report.scanned_files, 0);
                    assert_eq!(report.duplicate_groups.len(), 0);
        }

    #[test]
        fn protected_item_is_suggested_keep() {
                    let dir = tempdir().unwrap();
                    let archive = dir.path().join("archive");
                    fs::create_dir(&archive).unwrap();
                    write(dir.path(), "copy.txt", b"shared content here");
                    write(&archive, "original.txt", b"shared content here");
                    let report = scan_exact(&base_config(
                                    vec![dir.path().to_path_buf()],
                                    vec![archive],
                                )).unwrap();
                    assert_eq!(report.duplicate_groups.len(), 1);
                    let keep = report.duplicate_groups[0].items.iter().find(|i| i.suggested_keep).unwrap();
                    assert!(keep.is_protected);
        }

                            #[test]
        fn scanned_files_count_matches_actual_files() {
                    let dir = tempdir().unwrap();
                    write(dir.path(), "a.txt", b"aaa");
                    write(dir.path(), "b.txt", b"bbb");
                    write(dir.path(), "c.txt", b"ccc");
                    let report = scan_exact(&base_config(vec![dir.path().to_path_buf()], vec![])).unwrap();
                    assert_eq!(report.scanned_files, 3);
        }

    #[test]
        fn algorithm_label_appears_in_group() {
                    let dir = tempdir().unwrap();
                    write(dir.path(), "a.txt", b"same");
                    write(dir.path(), "b.txt", b"same");
                    let report = scan_exact(&base_config(vec![dir.path().to_path_buf()], vec![])).unwrap();
                    assert_eq!(report.duplicate_groups[0].algorithm, "blake3");
        }

    #[test]
        fn byte_verify_mode_finds_duplicates_and_sets_reason() {
                    let dir = tempdir().unwrap();
                    write(dir.path(), "a.txt", b"verified content");
                    write(dir.path(), "b.txt", b"verified content");
                    let mut config = base_config(vec![dir.path().to_path_buf()], vec![]);
                    config.byte_verify = true;
                    let report = scan_exact(&config).unwrap();
                    assert_eq!(report.duplicate_groups.len(), 1);
                    assert!(report.duplicate_groups[0].reason.contains("byte-by-byte verified"));
        }

    #[test]
        fn multiple_distinct_groups_all_reported() {
                    let dir = tempdir().unwrap();
                    write(dir.path(), "a1.txt", b"group one content");
                    write(dir.path(), "a2.txt", b"group one content");
                    write(dir.path(), "b1.txt", b"group two different");
                    write(dir.path(), "b2.txt", b"group two different");
                    let report = scan_exact(&base_config(vec![dir.path().to_path_buf()], vec![])).unwrap();
                    assert_eq!(report.duplicate_groups.len(), 2);
        }

    #[test]
        fn groups_sorted_by_size_descending() {
                    let dir = tempdir().unwrap();
                    write(dir.path(), "big_a.txt",   b"bigger file content here!!");
                    write(dir.path(), "big_b.txt",   b"bigger file content here!!");
                    write(dir.path(), "small_a.txt", b"tiny");
                    write(dir.path(), "small_b.txt", b"tiny");
                    let report = scan_exact(&base_config(vec![dir.path().to_path_buf()], vec![])).unwrap();
                    assert_eq!(report.duplicate_groups.len(), 2);
                    assert!(report.duplicate_groups[0].size >= report.duplicate_groups[1].size);
        }
}
