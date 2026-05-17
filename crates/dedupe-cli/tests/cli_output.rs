use std::path::PathBuf;
use std::process::Command;

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn json_output_includes_duplicate_group_and_keep_marker() {
    let root = fixture_dir("exact-basic");
    let protected = root.join("archive");

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
}

#[test]
fn csv_output_contains_expected_headers() {
    let root = fixture_dir("exact-basic");

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--output")
        .arg("csv")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.starts_with("group,suggested_keep,protected,size,algorithm,hash,reason,path")
    );
    assert!(stdout.contains("same size + same full hash"));
}
