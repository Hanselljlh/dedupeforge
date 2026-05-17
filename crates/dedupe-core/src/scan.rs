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
    let partial_candidates = hash_files(
        size_candidate_files,
        config.algorithm,
        config.partial_bytes,
        true,
        &mut errors,
    );
    let partial_candidate_files: Vec<FileEntry> =
        partial_candidates.into_values().flatten().collect();
    let full_candidates = hash_files(
        partial_candidate_files,
        config.algorithm,
        0,
        false,
        &mut errors,
    );

    let mut duplicate_groups = Vec::new();

    for ((_size, full_hash), mut group) in full_candidates {
        if group.len() < 2 {
            continue;
        }
        group.sort_by_key(item_sort_key);

        if config.byte_verify {
            for verified in split_by_byte_equality(&group, &mut errors) {
                if verified.len() > 1 {
                    duplicate_groups.push(make_group(
                        verified,
                        config.algorithm,
                        full_hash.clone(),
                        true,
                    ));
                }
            }
        } else {
            duplicate_groups.push(make_group(group, config.algorithm, full_hash, false));
        }
    }

    duplicate_groups.sort_by(|a, b| {
        b.size
            .cmp(&a.size)
            .then_with(|| a.items[0].path.cmp(&b.items[0].path))
    });

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
            Some(hash) if hash.starts_with("ERROR::") => errors.push(format!(
                "{}: {}",
                file.path.display(),
                hash.trim_start_matches("ERROR::")
            )),
            Some(hash) => {
                map.entry((file.size, hash)).or_default().push(file);
            }
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
                Err(e) => errors.push(format!(
                    "byte verify failed for {}: {e}",
                    item.path.display()
                )),
            }
        }
        verified_groups.push(vec![item]);
    }

    verified_groups
}

fn make_group(
    mut files: Vec<FileEntry>,
    algorithm: HashAlgorithm,
    hash: String,
    byte_verified: bool,
) -> DuplicateGroup {
    files.sort_by_key(item_sort_key);
    let keep_index = choose_keep_index(&files);
    let items = files
        .into_iter()
        .enumerate()
        .map(|(idx, f)| DuplicateItem {
            path: f.path,
            size: f.size,
            modified_unix: f.modified_unix,
            is_protected: f.is_protected,
            suggested_keep: idx == keep_index,
        })
        .collect::<Vec<_>>();

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
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("dedupeforge-scan-{unique}-{name}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn exact_scan_reports_duplicate_group_and_keep_candidate() {
        let root = temp_dir("scan-basic");
        let archive = root.join("archive");
        let current = root.join("current");
        fs::create_dir_all(&archive).unwrap();
        fs::create_dir_all(&current).unwrap();

        fs::write(archive.join("photo.jpg"), b"same-image-bytes").unwrap();
        fs::write(current.join("photo-copy.jpg"), b"same-image-bytes").unwrap();
        fs::write(current.join("unique.jpg"), b"unique-content!").unwrap();

        let config = ScanConfig {
            paths: vec![root.clone()],
            protected_roots: vec![archive.clone()],
            algorithm: HashAlgorithm::Blake3,
            partial_bytes: 4,
            min_size: 1,
            ignore_hidden: true,
            byte_verify: false,
        };

        let report = scan_exact(&config).unwrap();

        assert_eq!(report.scanned_files, 3);
        assert_eq!(report.candidate_size_groups, 1);
        assert_eq!(report.duplicate_groups.len(), 1);
        assert!(report.errors.is_empty());

        let group = &report.duplicate_groups[0];
        assert_eq!(group.reason, "same size + same full hash");
        assert_eq!(group.items.len(), 2);
        assert_eq!(group.items.iter().filter(|i| i.suggested_keep).count(), 1);
        assert!(group
            .items
            .iter()
            .any(|i| i.is_protected && i.suggested_keep));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn byte_verification_reason_is_reported_when_enabled() {
        let root = temp_dir("scan-verify");
        fs::write(root.join("a.bin"), b"verified-content").unwrap();
        fs::write(root.join("b.bin"), b"verified-content").unwrap();

        let config = ScanConfig {
            paths: vec![root.clone()],
            protected_roots: vec![],
            algorithm: HashAlgorithm::Sha256,
            partial_bytes: 8,
            min_size: 1,
            ignore_hidden: true,
            byte_verify: true,
        };

        let report = scan_exact(&config).unwrap();

        assert_eq!(report.duplicate_groups.len(), 1);
        assert_eq!(
            report.duplicate_groups[0].reason,
            "same size + same full hash + byte-by-byte verified"
        );

        fs::remove_dir_all(root).unwrap();
    }
}
