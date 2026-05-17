use crate::hash::HashAlgorithm;
use anyhow::Result;
use serde::{Deserialize, Serialize};
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

pub fn scan_exact(_config: &ScanConfig) -> Result<ScanReport> {
    Ok(ScanReport {
        scanned_files: 0,
        candidate_size_groups: 0,
        duplicate_groups: Vec::new(),
        errors: vec!["scan pipeline placeholder: add full implementation from generated package".to_string()],
    })
}
