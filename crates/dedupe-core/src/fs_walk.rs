use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::{DirEntry, WalkDir};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub modified_unix: Option<i64>,
    pub is_protected: bool,
}

pub fn collect_files(paths: &[PathBuf], protected_roots: &[PathBuf], ignore_hidden: bool) -> Result<Vec<FileEntry>> {
    let protected_roots = canonicalize_existing_roots(protected_roots);
    let mut files = Vec::new();

    for root in paths {
        let walker = WalkDir::new(root).follow_links(false).into_iter().filter_entry(|entry| {
            if !ignore_hidden { return true; }
            !is_hidden_entry(entry)
        });

        for entry in walker {
            let entry = entry.with_context(|| format!("failed to read entry under {}", root.display()))?;
            if !entry.file_type().is_file() { continue; }

            let metadata = entry.metadata().with_context(|| format!("failed to stat {}", entry.path().display()))?;
            let canonical_path = fs::canonicalize(entry.path()).unwrap_or_else(|_| entry.path().to_path_buf());
            let modified_unix = metadata.modified().ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);
            let is_protected = protected_roots.iter().any(|p| canonical_path.starts_with(p));

            files.push(FileEntry {
                path: canonical_path,
                size: metadata.len(),
                modified_unix,
                is_protected,
            });
        }
    }

    Ok(files)
}

fn canonicalize_existing_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    roots.iter()
        .map(|p| fs::canonicalize(p).unwrap_or_else(|_| p.clone()))
        .collect()
}

fn is_hidden_entry(entry: &DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|name| name.starts_with('.') || name.eq_ignore_ascii_case("thumbs.db") || name.eq_ignore_ascii_case("desktop.ini"))
        .unwrap_or(false)
}

#[allow(dead_code)]
fn is_under_path(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}
