use std::path::PathBuf;
use std::process::Command;

fn write_sample_png(path: &std::path::Path, seed: u8) {
    use image::{ImageBuffer, Rgb};

    let mut image = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(32, 32);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let value = seed
            .wrapping_add((x as u8).wrapping_mul(3))
            .wrapping_add((y as u8).wrapping_mul(5));
        *pixel = Rgb([value, value / 2, 255u8.wrapping_sub(value)]);
    }
    image.save(path).unwrap();
}

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
        stdout.starts_with("mode,group,suggested_keep,protected,size,algorithm,hash,reason,path")
    );
    assert!(stdout.contains("same size + same full hash"));
}

#[test]
fn profile_can_enable_cache_and_json_output() {
    let root = fixture_dir("exact-basic");
    let profile_path = std::env::temp_dir().join("dedupeforge-test-profile.json");
    let cache_path = std::env::temp_dir().join("dedupeforge-test-cache.sqlite3");
    let profile_json = format!(
        r#"{{
  "paths": ["{}"],
  "cache": true,
  "cache_path": "{}",
  "cache_mtime_tolerance_secs": 2,
  "output": "json"
}}"#,
        root.display().to_string().replace('\\', "\\\\"),
        cache_path.display().to_string().replace('\\', "\\\\")
    );
    std::fs::write(&profile_path, profile_json).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg("--profile")
        .arg(profile_path.as_os_str())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"cache_hits\""));

    let _ = std::fs::remove_file(profile_path);
    let _ = std::fs::remove_file(cache_path);
}

#[test]
fn preset_can_enable_network_tolerant_cache_mode() {
    let root = fixture_dir("exact-basic");

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--preset")
        .arg("network-tolerant")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Cache hits:"));
    assert!(stdout.contains("Cache misses:"));
}

#[test]
fn similar_name_mode_reports_high_risk_matches() {
    let root = std::env::temp_dir().join("dedupeforge-cli-similar-names");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("Vacation 2024.jpg"), b"a").unwrap();
    std::fs::write(root.join("vacation-2024.JPG"), b"b").unwrap();
    std::fs::write(root.join("invoice.pdf"), b"c").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("similar-names")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Mode: similar-names"));
    assert!(stdout.contains("Match risk: high"));
    assert!(stdout.contains("Duplicate groups: 1"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn duplicate_folder_mode_respects_ignore_patterns() {
    let root = std::env::temp_dir().join("dedupeforge-cli-duplicate-folders");
    let _ = std::fs::remove_dir_all(&root);
    let left = root.join("left");
    let right = root.join("right");
    std::fs::create_dir_all(&left).unwrap();
    std::fs::create_dir_all(&right).unwrap();
    std::fs::write(left.join("photo.jpg"), b"same").unwrap();
    std::fs::write(right.join("photo.jpg"), b"diff").unwrap();
    std::fs::write(left.join("skip.tmp"), b"noise").unwrap();
    std::fs::write(right.join("skip.tmp"), b"other").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("duplicate-folders")
        .arg("--ignore-pattern")
        .arg("*.tmp")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Mode: duplicate-folders"));
    assert!(stdout.contains("Match risk: medium"));
    assert!(stdout.contains("file-tree overlap"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn similar_image_mode_reports_high_risk_matches() {
    let root = std::env::temp_dir().join("dedupeforge-cli-similar-images");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    write_sample_png(&root.join("a.png"), 12);
    std::fs::copy(root.join("a.png"), root.join("b.png")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("similar-images")
        .arg("--image-hamming-threshold")
        .arg("4")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Mode: similar-images"));
    assert!(stdout.contains("Match risk: high"));
    assert!(stdout.contains("perceptual hash distance"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn similar_image_mode_can_reuse_cached_image_hashes() {
    let root = std::env::temp_dir().join("dedupeforge-cli-similar-images-cache");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let cache_path = root.join("images-cache.sqlite3");
    write_sample_png(&root.join("a.png"), 22);
    std::fs::copy(root.join("a.png"), root.join("b.png")).unwrap();

    let first = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("similar-images")
        .arg("--cache")
        .arg("--cache-path")
        .arg(cache_path.as_os_str())
        .output()
        .unwrap();
    assert!(first.status.success());

    let second = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("similar-images")
        .arg("--cache")
        .arg("--cache-path")
        .arg(cache_path.as_os_str())
        .output()
        .unwrap();
    assert!(second.status.success());

    let stdout = String::from_utf8(second.stdout).unwrap();
    assert!(stdout.contains("Cache hits:"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn raw_jpeg_pair_mode_reports_medium_risk_matches() {
    let root = std::env::temp_dir().join("dedupeforge-cli-raw-jpeg");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("IMG_1000.CR2"), b"raw").unwrap();
    std::fs::write(root.join("IMG-1000.jpg"), b"jpeg").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("raw-jpeg-pairs")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Mode: raw-jpeg-pairs"));
    assert!(stdout.contains("Match risk: medium"));
    assert!(stdout.contains("RAW + JPEG pair"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn empty_file_mode_reports_zero_byte_files() {
    let root = std::env::temp_dir().join("dedupeforge-cli-empty-files");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("a.txt"), b"").unwrap();
    std::fs::write(root.join("b.txt"), b"").unwrap();
    std::fs::write(root.join("c.txt"), b"data").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("empty-files")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Mode: empty-files"));
    assert!(stdout.contains("Match risk: low"));
    assert!(stdout.contains("empty files ready for review"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn empty_folder_mode_reports_empty_directories() {
    let root = std::env::temp_dir().join("dedupeforge-cli-empty-folders");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("empty")).unwrap();
    std::fs::create_dir_all(root.join("non-empty")).unwrap();
    std::fs::write(root.join("non-empty").join("file.txt"), b"data").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("empty-folders")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Mode: empty-folders"));
    assert!(stdout.contains("Match risk: low"));
    assert!(stdout.contains("empty folders ready for review"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn large_file_mode_reports_large_files_review() {
    let root = std::env::temp_dir().join("dedupeforge-cli-large-files");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("small.bin"), b"1234").unwrap();
    std::fs::write(root.join("large.bin"), b"1234567890").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("large-files")
        .arg("--min-size")
        .arg("5")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Mode: large-files"));
    assert!(stdout.contains("Match risk: low"));
    assert!(stdout.contains("files at or above 5 bytes"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn bad_extension_mode_reports_extension_mismatches() {
    let root = std::env::temp_dir().join("dedupeforge-cli-bad-extensions");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("photo.txt"), b"\x89PNG\r\n\x1A\nrest").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("bad-extensions")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Mode: bad-extensions"));
    assert!(stdout.contains("Match risk: medium"));
    assert!(stdout.contains("extension does not match detected content"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn similar_video_mode_reports_missing_dependency_clearly() {
    let root = fixture_dir("exact-basic");

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("similar-videos")
        .arg("--cache")
        .env("DEDUPEFORGE_FFMPEG", "definitely-not-ffmpeg")
        .env("DEDUPEFORGE_FFPROBE", "definitely-not-ffprobe")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Mode: similar-videos"));
    assert!(stdout.contains("media dependency check failed"));
}

#[test]
fn similar_audio_mode_reports_missing_dependency_clearly() {
    let root = fixture_dir("exact-basic");

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--mode")
        .arg("similar-audio")
        .arg("--cache")
        .env("DEDUPEFORGE_FFMPEG", "definitely-not-ffmpeg")
        .env("DEDUPEFORGE_FFPROBE", "definitely-not-ffprobe")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Mode: similar-audio"));
    assert!(stdout.contains("media dependency check failed"));
}

#[test]
fn clear_cache_succeeds_when_file_exists() {
    let cache_path = std::env::temp_dir().join("dedupeforge-clear-cache.sqlite3");
    std::fs::write(&cache_path, b"placeholder").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg("tests/fixtures/exact-basic")
        .arg("--clear-cache")
        .arg("--cache-path")
        .arg(cache_path.as_os_str())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!cache_path.exists());
}

#[test]
fn rebuild_cache_recreates_cache_before_scan() {
    let root = fixture_dir("exact-basic");
    let cache_path = std::env::temp_dir().join("dedupeforge-rebuild-cache.sqlite3");
    std::fs::write(&cache_path, b"stale").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--rebuild-cache")
        .arg("--cache")
        .arg("--cache-path")
        .arg(cache_path.as_os_str())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(cache_path.exists());

    let _ = std::fs::remove_file(cache_path);
}

#[test]
fn json_action_plan_output_contains_quarantine_move() {
    let root = fixture_dir("exact-basic");

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--action-plan")
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"action\": \"quarantine_move\""));
    assert!(stdout.contains("\"valid\": true"));
}

#[test]
fn execute_action_plan_moves_duplicate_into_quarantine() {
    let root = std::env::temp_dir().join("dedupeforge-cli-exec-action");
    let _ = std::fs::remove_dir_all(&root);
    let quarantine = root.join(".quarantine");
    let archive = root.join("archive");
    let current = root.join("current");
    std::fs::create_dir_all(&archive).unwrap();
    std::fs::create_dir_all(&current).unwrap();
    std::fs::write(archive.join("keep.txt"), b"same").unwrap();
    std::fs::write(current.join("copy.txt"), b"same").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--protected")
        .arg(archive.as_os_str())
        .arg("--action-plan")
        .arg("--execute-action-plan")
        .arg("--quarantine-root")
        .arg(quarantine.as_os_str())
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!current.join("copy.txt").exists());
    assert!(quarantine.exists());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn restore_manifest_restores_quarantined_duplicate() {
    let root = std::env::temp_dir().join("dedupeforge-cli-restore-action");
    let _ = std::fs::remove_dir_all(&root);
    let quarantine = root.join(".quarantine");
    let archive = root.join("archive");
    let current = root.join("current");
    std::fs::create_dir_all(&archive).unwrap();
    std::fs::create_dir_all(&current).unwrap();
    std::fs::write(archive.join("keep.txt"), b"same").unwrap();
    std::fs::write(current.join("copy.txt"), b"same").unwrap();

    let execute = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--protected")
        .arg(archive.as_os_str())
        .arg("--action-plan")
        .arg("--execute-action-plan")
        .arg("--quarantine-root")
        .arg(quarantine.as_os_str())
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    assert!(execute.status.success());
    assert!(!current.join("copy.txt").exists());

    let batch_dir = std::fs::read_dir(&quarantine)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.path().is_dir())
        .unwrap()
        .path();
    let manifest = batch_dir.join("manifest.json");

    let restore = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg("--restore-manifest")
        .arg(manifest.as_os_str())
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    assert!(restore.status.success());
    assert!(current.join("copy.txt").exists());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn save_action_plan_writes_plan_file() {
    let root = fixture_dir("exact-basic");
    let plan_path = std::env::temp_dir().join("dedupeforge-saved-plan.json");
    let _ = std::fs::remove_file(&plan_path);

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--action-plan")
        .arg("--save-action-plan")
        .arg(plan_path.as_os_str())
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(plan_path.exists());

    let _ = std::fs::remove_file(plan_path);
}

#[test]
fn keep_newest_rule_changes_action_plan_selection_rule() {
    let root = fixture_dir("exact-basic");

    let output = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--action-plan")
        .arg("--selection-rule")
        .arg("keep-newest")
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"selection_rule\": \"keep-newest\""));
}

#[test]
fn load_action_plan_renders_saved_plan() {
    let root = fixture_dir("exact-basic");
    let plan_path = std::env::temp_dir().join("dedupeforge-load-plan.json");
    let _ = std::fs::remove_file(&plan_path);

    let save = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--action-plan")
        .arg("--save-action-plan")
        .arg(plan_path.as_os_str())
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();
    assert!(save.status.success());

    let load = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg("--load-action-plan")
        .arg(plan_path.as_os_str())
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    assert!(load.status.success());
    let stdout = String::from_utf8(load.stdout).unwrap();
    assert!(stdout.contains("\"action\": \"quarantine_move\""));

    let _ = std::fs::remove_file(plan_path);
}

#[test]
fn load_action_plan_can_execute_saved_plan() {
    let root = std::env::temp_dir().join("dedupeforge-load-exec-plan");
    let _ = std::fs::remove_dir_all(&root);
    let quarantine = root.join(".quarantine");
    let archive = root.join("archive");
    let current = root.join("current");
    let plan_path = root.join("plan.json");
    std::fs::create_dir_all(&archive).unwrap();
    std::fs::create_dir_all(&current).unwrap();
    std::fs::write(archive.join("keep.txt"), b"same").unwrap();
    std::fs::write(current.join("copy.txt"), b"same").unwrap();

    let save = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--protected")
        .arg(archive.as_os_str())
        .arg("--action-plan")
        .arg("--save-action-plan")
        .arg(plan_path.as_os_str())
        .output()
        .unwrap();
    assert!(save.status.success());

    let exec = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg("--load-action-plan")
        .arg(plan_path.as_os_str())
        .arg("--execute-action-plan")
        .arg("--quarantine-root")
        .arg(quarantine.as_os_str())
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    assert!(exec.status.success());
    assert!(!current.join("copy.txt").exists());
    assert!(quarantine.exists());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn load_action_plan_rejects_stale_saved_plan_on_execute() {
    let root = std::env::temp_dir().join("dedupeforge-load-stale-plan");
    let _ = std::fs::remove_dir_all(&root);
    let quarantine = root.join(".quarantine");
    let archive = root.join("archive");
    let current = root.join("current");
    let plan_path = root.join("plan.json");
    std::fs::create_dir_all(&archive).unwrap();
    std::fs::create_dir_all(&current).unwrap();
    std::fs::write(archive.join("keep.txt"), b"same").unwrap();
    std::fs::write(current.join("copy.txt"), b"same").unwrap();

    let save = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--protected")
        .arg(archive.as_os_str())
        .arg("--action-plan")
        .arg("--save-action-plan")
        .arg(plan_path.as_os_str())
        .output()
        .unwrap();
    assert!(save.status.success());

    std::fs::remove_file(current.join("copy.txt")).unwrap();

    let exec = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg("--load-action-plan")
        .arg(plan_path.as_os_str())
        .arg("--execute-action-plan")
        .arg("--quarantine-root")
        .arg(quarantine.as_os_str())
        .output()
        .unwrap();

    assert!(!exec.status.success());
    let stderr = String::from_utf8(exec.stderr).unwrap();
    assert!(stderr.contains("action plan validation failed"));
    assert!(stderr.contains("is unavailable"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn report_database_can_store_and_list_reports() {
    let root = fixture_dir("exact-basic");
    let report_db = std::env::temp_dir().join("dedupeforge-report-db.sqlite3");
    let _ = std::fs::remove_file(&report_db);

    let scan = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg(root.as_os_str())
        .arg("--report-db")
        .arg(report_db.as_os_str())
        .arg("--store-report-name")
        .arg("nightly-fixture")
        .output()
        .unwrap();

    assert!(scan.status.success());

    let list = Command::new(env!("CARGO_BIN_EXE_dedupeforge"))
        .arg("--report-db")
        .arg(report_db.as_os_str())
        .arg("--list-report-db")
        .output()
        .unwrap();

    assert!(list.status.success());
    let stdout = String::from_utf8(list.stdout).unwrap();
    assert!(stdout.contains("nightly-fixture"));

    let _ = std::fs::remove_file(report_db);
}
