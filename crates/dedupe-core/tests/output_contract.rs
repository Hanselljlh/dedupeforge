use dedupe_core::scan::{DuplicateGroup, DuplicateItem, MatchRisk, ScanMode, ScanReport};
use std::path::PathBuf;

fn minimal_report() -> ScanReport {
    ScanReport {
        mode: ScanMode::Exact,
        scanned_files: 2,
        candidate_size_groups: 1,
        cache_hits: 0,
        cache_misses: 2,
        duplicate_groups: vec![DuplicateGroup {
            size: 100,
            algorithm: "blake3".to_string(),
            hash: "deadbeef".to_string(),
            reason: "same size + same full hash".to_string(),
            items: vec![
                DuplicateItem {
                    path: PathBuf::from("/keep/file.txt"),
                    size: 100,
                    modified_unix: Some(1000),
                    is_protected: true,
                    suggested_keep: true,
                },
                DuplicateItem {
                    path: PathBuf::from("/dup/file.txt"),
                    size: 100,
                    modified_unix: Some(2000),
                    is_protected: false,
                    suggested_keep: false,
                },
            ],
        }],
        errors: vec![],
        risk: MatchRisk::Low,
    }
}

#[test]
fn json_top_level_fields_are_present() {
    let json: serde_json::Value = serde_json::to_value(minimal_report()).unwrap();
    for field in [
        "mode",
        "scanned_files",
        "candidate_size_groups",
        "cache_hits",
        "cache_misses",
        "duplicate_groups",
        "errors",
        "risk",
    ] {
        assert!(json.get(field).is_some(), "missing top-level field: {field}");
    }
}

#[test]
fn json_mode_uses_kebab_case() {
    let mut r = minimal_report();
    r.mode = ScanMode::SimilarImages;
    let json: serde_json::Value = serde_json::to_value(&r).unwrap();
    assert_eq!(json["mode"], "similar-images");

    r.mode = ScanMode::Exact;
    let json: serde_json::Value = serde_json::to_value(&r).unwrap();
    assert_eq!(json["mode"], "exact");

    r.mode = ScanMode::RawJpegPairs;
    let json: serde_json::Value = serde_json::to_value(&r).unwrap();
    assert_eq!(json["mode"], "raw-jpeg-pairs");

    r.mode = ScanMode::DuplicateArchiveMembers;
    let json: serde_json::Value = serde_json::to_value(&r).unwrap();
    assert_eq!(json["mode"], "duplicate-archive-members");
}

#[test]
fn json_risk_uses_lowercase() {
    let json: serde_json::Value = serde_json::to_value(minimal_report()).unwrap();
    assert_eq!(json["risk"], "low");

    let mut r = minimal_report();
    r.risk = MatchRisk::Medium;
    let json: serde_json::Value = serde_json::to_value(&r).unwrap();
    assert_eq!(json["risk"], "medium");

    r.risk = MatchRisk::High;
    let json: serde_json::Value = serde_json::to_value(&r).unwrap();
    assert_eq!(json["risk"], "high");
}

#[test]
fn json_group_fields_are_present_and_correct() {
    let json: serde_json::Value = serde_json::to_value(minimal_report()).unwrap();
    let group = &json["duplicate_groups"][0];
    assert_eq!(group["size"], 100);
    assert_eq!(group["algorithm"], "blake3");
    assert_eq!(group["hash"], "deadbeef");
    assert_eq!(group["reason"], "same size + same full hash");
    assert!(group.get("items").is_some());
}

#[test]
fn json_item_fields_are_present_and_correct() {
    let json: serde_json::Value = serde_json::to_value(minimal_report()).unwrap();
    let keep_item = &json["duplicate_groups"][0]["items"][0];
    assert!(keep_item.get("path").is_some());
    assert_eq!(keep_item["size"], 100);
    assert_eq!(keep_item["is_protected"], true);
    assert_eq!(keep_item["suggested_keep"], true);
    assert_eq!(keep_item["modified_unix"], 1000);

    let dup_item = &json["duplicate_groups"][0]["items"][1];
    assert_eq!(dup_item["is_protected"], false);
    assert_eq!(dup_item["suggested_keep"], false);
}

fn csv_bytes(report: &ScanReport) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut w = csv::Writer::from_writer(&mut buf);
        w.write_record([
            "group",
            "suggested_keep",
            "protected",
            "size",
            "algorithm",
            "hash",
            "reason",
            "path",
        ])
        .unwrap();
        for (gi, group) in report.duplicate_groups.iter().enumerate() {
            for item in &group.items {
                w.write_record([
                    (gi + 1).to_string(),
                    item.suggested_keep.to_string(),
                    item.is_protected.to_string(),
                    item.size.to_string(),
                    group.algorithm.clone(),
                    group.hash.clone(),
                    group.reason.clone(),
                    item.path.display().to_string(),
                ])
                .unwrap();
            }
        }
        w.flush().unwrap();
    }
    buf
}

#[test]
fn csv_output_has_correct_column_headers() {
    let content = String::from_utf8(csv_bytes(&minimal_report())).unwrap();
    let mut rdr = csv::Reader::from_reader(content.as_bytes());
    let headers = rdr.headers().unwrap().clone();
    assert_eq!(
        headers,
        csv::StringRecord::from(vec![
            "group",
            "suggested_keep",
            "protected",
            "size",
            "algorithm",
            "hash",
            "reason",
            "path",
        ])
    );
}

#[test]
fn csv_output_row_values_match_report() {
    let content = String::from_utf8(csv_bytes(&minimal_report())).unwrap();
    let mut rdr = csv::Reader::from_reader(content.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();

    assert_eq!(records.len(), 2);

    // keep item
    assert_eq!(&records[0][0], "1");          // group
    assert_eq!(&records[0][1], "true");       // suggested_keep
    assert_eq!(&records[0][2], "true");       // protected
    assert_eq!(&records[0][3], "100");        // size
    assert_eq!(&records[0][4], "blake3");     // algorithm
    assert_eq!(&records[0][5], "deadbeef");   // hash
    assert_eq!(&records[0][6], "same size + same full hash"); // reason

    // dup item
    assert_eq!(&records[1][0], "1");          // same group
    assert_eq!(&records[1][1], "false");      // suggested_keep
    assert_eq!(&records[1][2], "false");      // protected
    assert_eq!(&records[1][3], "100");        // size
}

#[test]
fn csv_empty_report_produces_only_header_row() {
    let mut report = minimal_report();
    report.duplicate_groups.clear();
    let content = String::from_utf8(csv_bytes(&report)).unwrap();
    let mut rdr = csv::Reader::from_reader(content.as_bytes());
    assert_eq!(rdr.records().count(), 0);
}
