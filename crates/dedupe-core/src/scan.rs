use crate::fs_walk::{collect_files, FileEntry, FileIdentity};
use crate::hash::{hash_file, hash_file_prefix, HashAlgorithm};
use crate::verify::files_equal;
use anyhow::Result;
use dedupe_cache::{Cache, CacheFileIdentity, CacheLookupPolicy, HashScope};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanConfig {
    pub paths: Vec<PathBuf>,
    pub protected_roots: Vec<PathBuf>,
    pub algorithm: HashAlgorithm,
    pub partial_bytes: u64,
    pub min_size: u64,
    pub ignore_hidden: bool,
    pub byte_verify: bool,
    pub cache: CacheConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub path: Option<PathBuf>,
    pub modified_time_tolerance_secs: i64,
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
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub errors: Vec<String>,
}

pub fn scan_exact(config: &ScanConfig) -> Result<ScanReport> {
    let cache = open_cache_if_enabled(&config.cache)?;
    let collected = collect_files(&config.paths, &config.protected_roots, config.ignore_hidden)?;
    let files = collected
        .files
        .into_iter()
        .filter(|f| f.size >= config.min_size)
        .collect::<Vec<_>>();

    let scanned_files = files.len();
    let by_size = group_by_size(files);
    let candidate_size_groups = by_size.values().filter(|g| g.len() > 1).count();

    let mut errors = collected.errors;
    let mut cache_hits = 0usize;
    let mut cache_misses = 0usize;
    let size_candidate_files: Vec<FileEntry> = by_size.into_values().flatten().collect();
    let partial_candidates = hash_files(
        cache.as_ref(),
        size_candidate_files,
        config.algorithm,
        config.partial_bytes,
        true,
        config.cache.modified_time_tolerance_secs,
        &mut errors,
        &mut cache_hits,
        &mut cache_misses,
    );
    let partial_candidate_files: Vec<FileEntry> =
        partial_candidates.into_values().flatten().collect();
    let full_candidates = hash_files(
        cache.as_ref(),
        partial_candidate_files,
        config.algorithm,
        0,
        false,
        config.cache.modified_time_tolerance_secs,
        &mut errors,
        &mut cache_hits,
        &mut cache_misses,
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
        cache_hits,
        cache_misses,
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
    cache: Option<&Cache>,
    files: Vec<FileEntry>,
    algorithm: HashAlgorithm,
    partial_bytes: u64,
    partial: bool,
    modified_time_tolerance_secs: i64,
    errors: &mut Vec<String>,
    cache_hits: &mut usize,
    cache_misses: &mut usize,
) -> HashMap<(u64, String), Vec<FileEntry>> {
    let hashed: Vec<(FileEntry, Result<(String, bool), String>)> = if let Some(cache) = cache {
        files
            .into_iter()
            .map(|file| {
                let hash_result = hash_with_cache(
                    Some(cache),
                    &file,
                    algorithm,
                    partial_bytes,
                    partial,
                    modified_time_tolerance_secs,
                );
                match hash_result {
                    Ok((hash, used_cache)) => (file, Ok((hash, used_cache))),
                    Err(e) => (file, Err(e.to_string())),
                }
            })
            .collect()
    } else {
        files
            .into_par_iter()
            .map(|file| {
                let hash_result = hash_without_cache(&file, algorithm, partial_bytes, partial);
                match hash_result {
                    Ok((hash, used_cache)) => (file, Ok((hash, used_cache))),
                    Err(e) => (file, Err(e.to_string())),
                }
            })
            .collect()
    };

    let mut map: HashMap<(u64, String), Vec<FileEntry>> = HashMap::new();

    for (file, maybe_hash) in hashed {
        match maybe_hash {
            Err(error) => errors.push(format!("{}: {error}", file.path.display())),
            Ok((hash, used_cache)) => {
                if used_cache {
                    *cache_hits += 1;
                } else {
                    *cache_misses += 1;
                }
                map.entry((file.size, hash)).or_default().push(file);
            }
        }
    }

    map.retain(|_, group| group.len() > 1);
    map
}

fn hash_without_cache(
    file: &FileEntry,
    algorithm: HashAlgorithm,
    partial_bytes: u64,
    partial: bool,
) -> Result<(String, bool)> {
    let hash = if partial {
        hash_file_prefix(&file.path, algorithm, partial_bytes)?
    } else {
        hash_file(&file.path, algorithm)?
    };
    Ok((hash, false))
}

fn hash_with_cache(
    cache: Option<&Cache>,
    file: &FileEntry,
    algorithm: HashAlgorithm,
    partial_bytes: u64,
    partial: bool,
    modified_time_tolerance_secs: i64,
) -> Result<(String, bool)> {
    let scope = if partial {
        HashScope::Partial {
            bytes_hashed: partial_bytes,
        }
    } else {
        HashScope::Full
    };
    let label = algorithm.label();

    if let Some(cache) = cache {
        if let Some(found) =
            cache.lookup_hash(
                &file.path,
                cache_identity(file).as_ref(),
                file.size,
                file.modified_unix,
                label,
                scope,
                CacheLookupPolicy {
                    modified_time_tolerance_secs,
                },
            )?
        {
            cache.mark_seen(&file.path)?;
            return Ok((found.hash, true));
        }
    }

    let hash = if partial {
        hash_file_prefix(&file.path, algorithm, partial_bytes)?
    } else {
        hash_file(&file.path, algorithm)?
    };

    if let Some(cache) = cache {
        let identity = cache_identity(file);
        cache.store_hash(
            &file.path,
            identity.as_ref(),
            file.size,
            file.modified_unix,
            label,
            scope,
            &hash,
        )?;
    }

    Ok((hash, false))
}

fn cache_identity(file: &FileEntry) -> Option<CacheFileIdentity> {
    file.identity
        .as_ref()
        .map(|identity| cache_identity_from_file_identity(identity))
}

fn cache_identity_from_file_identity(identity: &FileIdentity) -> CacheFileIdentity {
    CacheFileIdentity {
        device_id: identity.device_id.clone(),
        inode: identity.inode.clone(),
    }
}

fn open_cache_if_enabled(config: &CacheConfig) -> Result<Option<Cache>> {
    if !config.enabled {
        return Ok(None);
    }

    let path = config
        .path
        .as_deref()
        .unwrap_or_else(|| Path::new(".dedupeforge-cache.sqlite3"));
    Ok(Some(Cache::open(path)?))
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
            cache: CacheConfig {
                enabled: false,
                path: None,
                modified_time_tolerance_secs: 0,
            },
        };

        let report = scan_exact(&config).unwrap();

        assert_eq!(report.scanned_files, 3);
        assert_eq!(report.candidate_size_groups, 1);
        assert_eq!(report.cache_hits, 0);
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
            cache: CacheConfig {
                enabled: false,
                path: None,
                modified_time_tolerance_secs: 0,
            },
        };

        let report = scan_exact(&config).unwrap();

        assert_eq!(report.duplicate_groups.len(), 1);
        assert_eq!(
            report.duplicate_groups[0].reason,
            "same size + same full hash + byte-by-byte verified"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn second_scan_reuses_cached_hashes() {
        let root = temp_dir("scan-cache");
        let cache_path = root.join("cache.sqlite3");
        fs::write(root.join("a.bin"), b"same-bytes").unwrap();
        fs::write(root.join("b.bin"), b"same-bytes").unwrap();

        let config = ScanConfig {
            paths: vec![root.clone()],
            protected_roots: vec![],
            algorithm: HashAlgorithm::Blake3,
            partial_bytes: 4,
            min_size: 1,
            ignore_hidden: true,
            byte_verify: false,
            cache: CacheConfig {
                enabled: true,
                path: Some(cache_path),
                modified_time_tolerance_secs: 0,
            },
        };

        let first = scan_exact(&config).unwrap();
        let second = scan_exact(&config).unwrap();

        assert_eq!(first.cache_hits, 0);
        assert!(second.cache_hits >= 2);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scan_continues_when_hashing_one_candidate_fails() {
        let root = temp_dir("hash-error");
        let archive = root.join("archive");
        let current = root.join("current");
        fs::create_dir_all(&archive).unwrap();
        fs::create_dir_all(&current).unwrap();

        let keep = archive.join("keep.txt");
        let copy = current.join("copy.txt");
        let missing = current.join("missing.txt");

        fs::write(&keep, b"same").unwrap();
        fs::write(&copy, b"same").unwrap();
        fs::write(&missing, b"same").unwrap();

        let mut candidate_files = vec![
            FileEntry {
                path: keep.clone(),
                size: 4,
                modified_unix: Some(1),
                identity: None,
                is_protected: true,
            },
            FileEntry {
                path: copy.clone(),
                size: 4,
                modified_unix: Some(2),
                identity: None,
                is_protected: false,
            },
            FileEntry {
                path: missing.clone(),
                size: 4,
                modified_unix: Some(3),
                identity: None,
                is_protected: false,
            },
        ];

        fs::remove_file(&missing).unwrap();

        let mut errors = Vec::new();
        let mut cache_hits = 0;
        let mut cache_misses = 0;
        let hashed = hash_files(
            None,
            std::mem::take(&mut candidate_files),
            HashAlgorithm::Blake3,
            0,
            false,
            0,
            &mut errors,
            &mut cache_hits,
            &mut cache_misses,
        );

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("missing.txt"));
        assert_eq!(hashed.len(), 1);
        assert_eq!(hashed.values().next().unwrap().len(), 2);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cache_reuses_hashes_when_modified_time_is_within_tolerance() {
        let root = temp_dir("scan-cache-tolerance");
        let cache_path = root.join("cache.sqlite3");
        let first = root.join("first.bin");
        let second = root.join("second.bin");
        fs::write(&first, b"same-bytes").unwrap();
        fs::write(&second, b"same-bytes").unwrap();

        let base = ScanConfig {
            paths: vec![root.clone()],
            protected_roots: vec![],
            algorithm: HashAlgorithm::Blake3,
            partial_bytes: 4,
            min_size: 1,
            ignore_hidden: true,
            byte_verify: false,
            cache: CacheConfig {
                enabled: true,
                path: Some(cache_path.clone()),
                modified_time_tolerance_secs: 0,
            },
        };

        let _ = scan_exact(&base).unwrap();

        let cache = Cache::open(&cache_path).unwrap();
        cache
            .store_hash(
                &first,
                None,
                10,
                Some(100),
                "blake3",
                HashScope::Partial { bytes_hashed: 4 },
                "synthetic-partial",
            )
            .unwrap();
        cache
            .store_hash(
                &second,
                None,
                10,
                Some(100),
                "blake3",
                HashScope::Partial { bytes_hashed: 4 },
                "synthetic-partial",
            )
            .unwrap();
        cache
            .store_hash(
                &first,
                None,
                10,
                Some(100),
                "blake3",
                HashScope::Full,
                "synthetic-full",
            )
            .unwrap();
        cache
            .store_hash(
                &second,
                None,
                10,
                Some(100),
                "blake3",
                HashScope::Full,
                "synthetic-full",
            )
            .unwrap();

        let candidate_files = vec![
            FileEntry {
                path: first,
                size: 10,
                modified_unix: Some(102),
                identity: None,
                is_protected: false,
            },
            FileEntry {
                path: second,
                size: 10,
                modified_unix: Some(102),
                identity: None,
                is_protected: false,
            },
        ];

        let mut errors = Vec::new();
        let mut cache_hits = 0;
        let mut cache_misses = 0;
        let hashed = hash_files(
            Some(&cache),
            candidate_files,
            HashAlgorithm::Blake3,
            4,
            true,
            2,
            &mut errors,
            &mut cache_hits,
            &mut cache_misses,
        );

        assert!(errors.is_empty());
        assert_eq!(cache_hits, 2);
        assert_eq!(cache_misses, 0);
        assert_eq!(hashed.len(), 1);

        drop(cache);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cache_reuses_hashes_when_path_changes_but_identity_matches() {
        let cache_path = temp_dir("scan-cache-rename").join("cache.sqlite3");
        let original = std::env::temp_dir().join("dedupeforge-rename-original.bin");
        let renamed = std::env::temp_dir().join("dedupeforge-rename-renamed.bin");
        fs::write(&original, b"same-bytes").unwrap();

        let cache = Cache::open(&cache_path).unwrap();
        let identity = CacheFileIdentity {
            device_id: "device-a".to_string(),
            inode: "inode-b".to_string(),
        };

        cache
            .store_hash(
                &original,
                Some(&identity),
                10,
                Some(100),
                "blake3",
                HashScope::Full,
                "identity-full",
            )
            .unwrap();

        let renamed_entry = FileEntry {
            path: renamed,
            size: 10,
            modified_unix: Some(100),
            identity: Some(FileIdentity {
                device_id: "device-a".to_string(),
                inode: "inode-b".to_string(),
            }),
            is_protected: false,
        };

        let mut errors = Vec::new();
        let mut cache_hits = 0;
        let mut cache_misses = 0;
        let hashed = hash_files(
            Some(&cache),
            vec![renamed_entry.clone(), renamed_entry],
            HashAlgorithm::Blake3,
            0,
            false,
            0,
            &mut errors,
            &mut cache_hits,
            &mut cache_misses,
        );

        assert!(errors.is_empty());
        assert_eq!(cache_hits, 2);
        assert_eq!(hashed.len(), 1);

        drop(cache);
        let _ = fs::remove_file(original);
        let _ = fs::remove_dir_all(cache_path.parent().unwrap());
    }
}
