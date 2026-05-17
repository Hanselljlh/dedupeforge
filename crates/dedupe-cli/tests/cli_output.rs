use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("dedupeforge-cli-{unique}-{name}"));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn json_output_includes_duplicate_group_and_keep_marker() {
    let root = temp_dir("json");
    let protected = root.join("archive");
    let current = root.join("current");
    fs::create_dir_all(&protected).unwrap();
    fs::create_dir_all(&current).unwrap();
    fs::write(protected.join("keep.txt"), b"same").unwrap();
    fs::write(current.join("copy.txt"), b"same").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--protected")
        .arg(protected.as_os_str())
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"duplicate_groups\""));
    assert!(stdout.contains("\"suggested_keep\": true"));
    assert!(stdout.contains("\"is_protected\": true"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn csv_output_contains_expected_headers() {
    let root = temp_dir("csv");
    fs::write(root.join("a.txt"), b"same").unwrap();
    fs::write(root.join("b.txt"), b"same").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--output")
        .arg("csv")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("group,suggested_keep,protected,size,algorithm,hash,reason,path"));
    assert!(stdout.contains("same size + same full hash"));

    fs::remove_dir_all(root).unwrap();
}
