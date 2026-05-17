use dedupe_core::hash::HashAlgorithm;
use dedupe_core::scan::{scan_exact, ScanConfig};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn base_config(paths: Vec<PathBuf>) -> ScanConfig {
    ScanConfig {
        paths,
        protected_roots: vec![],
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
fn exact_scan_finds_known_duplicates() {
      let dir = tempdir().unwrap();
      write(dir.path(), "photo_copy.jpg", b"fake jpeg data 12345");
      write(dir.path(), "photo_orig.jpg", b"fake jpeg data 12345");
      write(dir.path(), "readme.txt",     b"this file is unique");
      let report = scan_exact(&base_config(vec![dir.path().to_path_buf()])).unwrap();
      assert_eq!(report.scanned_files, 3);
      assert_eq!(report.duplicate_groups.len(), 1);
      assert_eq!(report.duplicate_groups[0].items.len(), 2);
      assert_eq!(report.errors.len(), 0);
}

#[test]
fn scan_across_multiple_input_dirs() {
      let dir_a = tempdir().unwrap();
      let dir_b = tempdir().unwrap();
      write(dir_a.path(), "file.bin", b"shared binary blob");
      write(dir_b.path(), "file.bin", b"shared binary blob");
      let report = scan_exact(&base_config(vec![
                dir_a.path().to_path_buf(),
                dir_b.path().to_path_buf(),
            ])).unwrap();
      assert_eq!(report.duplicate_groups.len(), 1);
}

#[test]
fn protected_folder_item_is_preferred_keep() {
      let dir = tempdir().unwrap();
      let archive = dir.path().join("archive");
      fs::create_dir(&archive).unwrap();
      write(dir.path(), "working_copy.dat", b"canonical file bytes");
      write(&archive,   "master.dat",       b"canonical file bytes");
      let report = scan_exact(&ScanConfig {
                paths: vec![dir.path().to_path_buf()],
                protected_roots: vec![archive],
                algorithm: HashAlgorithm::Blake3,
                partial_bytes: 4096,
                min_size: 1,
                ignore_hidden: false,
                byte_verify: false,
      }).unwrap();
      assert_eq!(report.duplicate_groups.len(), 1);
      let keep = report.duplicate_groups[0].items.iter().find(|i| i.suggested_keep).unwrap();
      assert!(keep.is_protected);
}

        #[test]
fn no_destructive_actions_taken() {
      let dir = tempdir().unwrap();
      let content = b"integrity check content";
      write(dir.path(), "check_a.txt", content);
      write(dir.path(), "check_b.txt", content);
      let _ = scan_exact(&base_config(vec![dir.path().to_path_buf()])).unwrap();
      assert_eq!(fs::read(dir.path().join("check_a.txt")).unwrap(), content);
      assert_eq!(fs::read(dir.path().join("check_b.txt")).unwrap(), content);
}

#[test]
fn all_three_algorithms_produce_correct_labels() {
      let dir = tempdir().unwrap();
      write(dir.path(), "x.bin", b"algo test bytes");
      write(dir.path(), "y.bin", b"algo test bytes");
      for (algo, label) in [
                (HashAlgorithm::Blake3,   "blake3"),
                (HashAlgorithm::Xxh3_128, "xxh3_128"),
                (HashAlgorithm::Sha256,   "sha256"),
            ] {
                let report = scan_exact(&ScanConfig {
                              paths: vec![dir.path().to_path_buf()],
                              protected_roots: vec![],
                              algorithm: algo,
                              partial_bytes: 4096,
                              min_size: 1,
                              ignore_hidden: false,
                              byte_verify: false,
                }).unwrap();
                assert_eq!(report.duplicate_groups.len(), 1);
                assert_eq!(report.duplicate_groups[0].algorithm, label);
      }
}

#[test]
fn hidden_files_excluded_when_flag_set() {
      let dir = tempdir().unwrap();
      write(dir.path(), ".hidden_a",   b"secret content");
      write(dir.path(), ".hidden_b",   b"secret content");
      write(dir.path(), "visible.txt", b"visible content");
      let report = scan_exact(&ScanConfig {
                paths: vec![dir.path().to_path_buf()],
                protected_roots: vec![],
                algorithm: HashAlgorithm::Blake3,
                partial_bytes: 4096,
                min_size: 1,
                ignore_hidden: true,
                byte_verify: false,
      }).unwrap();
      assert_eq!(report.scanned_files, 1, "hidden files must not be counted");
      assert_eq!(report.duplicate_groups.len(), 0);
}
