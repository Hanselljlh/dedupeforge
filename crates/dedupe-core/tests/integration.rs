use dedupe_core::hash::HashAlgorithm;
use dedupe_core::scan::{scan_exact, CacheConfig, ScanConfig, ScanMode};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("dedupe-core-integration-{unique}-{name}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn base_config(paths: Vec<PathBuf>) -> ScanConfig {
    ScanConfig {
        mode: ScanMode::Exact,
        paths,
        protected_roots: vec![],
        algorithm: HashAlgorithm::Blake3,
        partial_bytes: 4096,
        min_size: 1,
        ignore_hidden: false,
        byte_verify: false,
        cache: CacheConfig {
            enabled: false,
            path: None,
            modified_time_tolerance_secs: 0,
        },
        name_similarity_threshold: 85,
        folder_similarity_threshold: 85,
        image_hash_size: 8,
        image_hamming_threshold: 12,
        image_rotation_invariant: false,
        media_duration_tolerance_secs: 2.0,
        media_fingerprint_distance_threshold: 32,
        scan_archives: false,
        ignore_patterns: Vec::new(),
    }
}

fn write(dir: &Path, name: &str, content: &[u8]) {
    fs::write(dir.join(name), content).unwrap();
}

#[test]
fn exact_scan_finds_known_duplicates() {
    let dir = temp_dir("known-dupes");
    write(&dir, "photo_copy.jpg", b"fake jpeg data 12345");
    write(&dir, "photo_orig.jpg", b"fake jpeg data 12345");
    write(&dir, "readme.txt", b"this file is unique");

    let report = scan_exact(&base_config(vec![dir.clone()])).unwrap();
    assert_eq!(report.scanned_files, 3);
    assert_eq!(report.duplicate_groups.len(), 1);
    assert_eq!(report.duplicate_groups[0].items.len(), 2);
    assert!(report.errors.is_empty());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn scan_across_multiple_input_dirs() {
    let dir_a = temp_dir("multi-a");
    let dir_b = temp_dir("multi-b");
    write(&dir_a, "file.bin", b"shared binary blob");
    write(&dir_b, "file.bin", b"shared binary blob");

    let report = scan_exact(&base_config(vec![dir_a.clone(), dir_b.clone()])).unwrap();
    assert_eq!(report.duplicate_groups.len(), 1);

    let _ = fs::remove_dir_all(dir_a);
    let _ = fs::remove_dir_all(dir_b);
}

#[test]
fn protected_folder_item_is_preferred_keep() {
    let dir = temp_dir("protected");
    let archive = dir.join("archive");
    fs::create_dir(&archive).unwrap();
    write(&dir, "working_copy.dat", b"canonical file bytes");
    write(&archive, "master.dat", b"canonical file bytes");

    let mut config = base_config(vec![dir.clone()]);
    config.protected_roots = vec![archive];

    let report = scan_exact(&config).unwrap();
    assert_eq!(report.duplicate_groups.len(), 1);
    let keep = report.duplicate_groups[0]
        .items
        .iter()
        .find(|item| item.suggested_keep)
        .unwrap();
    assert!(keep.is_protected);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn no_destructive_actions_taken() {
    let dir = temp_dir("nondestructive");
    let content = b"integrity check content";
    write(&dir, "check_a.txt", content);
    write(&dir, "check_b.txt", content);

    let _ = scan_exact(&base_config(vec![dir.clone()])).unwrap();
    assert_eq!(fs::read(dir.join("check_a.txt")).unwrap(), content);
    assert_eq!(fs::read(dir.join("check_b.txt")).unwrap(), content);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn all_three_algorithms_produce_correct_labels() {
    let dir = temp_dir("algorithms");
    write(&dir, "x.bin", b"algo test bytes");
    write(&dir, "y.bin", b"algo test bytes");

    for (algorithm, label) in [
        (HashAlgorithm::Blake3, "blake3"),
        (HashAlgorithm::Xxh3_128, "xxh3_128"),
        (HashAlgorithm::Sha256, "sha256"),
    ] {
        let mut config = base_config(vec![dir.clone()]);
        config.algorithm = algorithm;
        let report = scan_exact(&config).unwrap();
        assert_eq!(report.duplicate_groups.len(), 1);
        assert_eq!(report.duplicate_groups[0].algorithm, label);
    }

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn hidden_files_excluded_when_flag_set() {
    let dir = temp_dir("hidden");
    write(&dir, ".hidden_a", b"secret content");
    write(&dir, ".hidden_b", b"secret content");
    write(&dir, "visible.txt", b"visible content");

    let mut config = base_config(vec![dir.clone()]);
    config.ignore_hidden = true;

    let report = scan_exact(&config).unwrap();
    assert_eq!(report.scanned_files, 1, "hidden files must not be counted");
    assert_eq!(report.duplicate_groups.len(), 0);

    let _ = fs::remove_dir_all(dir);
}
