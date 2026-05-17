use anyhow::Result;
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FileCollection {
    pub files: Vec<FileEntry>,
    pub errors: Vec<String>,
}

pub fn collect_files(
    paths: &[PathBuf],
    protected_roots: &[PathBuf],
    ignore_hidden: bool,
) -> Result<FileCollection> {
    let protected_roots = canonicalize_existing_roots(protected_roots);
    let mut files = Vec::new();
    let mut errors = Vec::new();

    for root in paths {
        if !root.exists() {
            anyhow::bail!("scan root does not exist: {}", root.display());
        }

        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                if !ignore_hidden {
                    return true;
                }
                !is_hidden_entry(entry)
            });

        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    errors.push(format!("failed to read entry under {}: {err}", root.display()));
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(err) => {
                    errors.push(format!("failed to stat {}: {err}", entry.path().display()));
                    continue;
                }
            };
            let canonical_path =
                fs::canonicalize(entry.path()).unwrap_or_else(|_| entry.path().to_path_buf());
            let modified_unix = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);
            let is_protected = protected_roots
                .iter()
                .any(|p| canonical_path.starts_with(p));

            files.push(FileEntry {
                path: canonical_path,
                size: metadata.len(),
                modified_unix,
                is_protected,
            });
        }
    }

    Ok(FileCollection { files, errors })
}

fn canonicalize_existing_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    roots
        .iter()
        .map(|p| fs::canonicalize(p).unwrap_or_else(|_| p.clone()))
        .collect()
}

fn is_hidden_entry(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|name| {
            name.starts_with('.')
                || name.eq_ignore_ascii_case("thumbs.db")
                || name.eq_ignore_ascii_case("desktop.ini")
        })
        .unwrap_or(false)
}

#[allow(dead_code)]
fn is_under_path(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("dedupeforge-walk-{unique}-{name}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn ignores_hidden_files_when_requested() {
        let root = temp_dir("hidden");
        let visible = root.join("visible.txt");
        let hidden = root.join(".hidden.txt");
        fs::write(&visible, b"visible").unwrap();
        fs::write(&hidden, b"hidden").unwrap();

        let collection = collect_files(std::slice::from_ref(&root), &[], true).unwrap();
        let paths: Vec<_> = collection.files.into_iter().map(|f| f.path).collect();

        assert!(paths.iter().any(|p| p.ends_with("visible.txt")));
        assert!(!paths.iter().any(|p| p.ends_with(".hidden.txt")));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn marks_files_under_protected_roots() {
        let root = temp_dir("protected-root");
        let protected = root.join("archive");
        let current = root.join("current");
        fs::create_dir_all(&protected).unwrap();
        fs::create_dir_all(&current).unwrap();
        fs::write(protected.join("keep.txt"), b"same").unwrap();
        fs::write(current.join("copy.txt"), b"same").unwrap();

        let files = collect_files(
            std::slice::from_ref(&root),
            std::slice::from_ref(&protected),
            true,
        )
        .unwrap();

        let protected_file = files
            .files
            .iter()
            .find(|f| f.path.ends_with("keep.txt"))
            .unwrap();
        let current_file = files
            .files
            .iter()
            .find(|f| f.path.ends_with("copy.txt"))
            .unwrap();

        assert!(protected_file.is_protected);
        assert!(!current_file.is_protected);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonicalizes_relative_protected_roots() {
        let root = temp_dir("relative-protected");
        let original_dir = std::env::current_dir().unwrap();
        let protected = root.join("archive");
        fs::create_dir_all(&protected).unwrap();
        fs::write(protected.join("keep.txt"), b"same").unwrap();

        std::env::set_current_dir(&root).unwrap();
        let files = collect_files(&[PathBuf::from("archive")], &[PathBuf::from("archive")], true)
            .unwrap();
        std::env::set_current_dir(original_dir).unwrap();

        let protected_file = files
            .files
            .iter()
            .find(|f| f.path.file_name().and_then(|name| name.to_str()) == Some("keep.txt"))
            .unwrap();
        assert!(protected_file.is_protected);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tolerates_nonexistent_protected_roots() {
        let root = temp_dir("missing-protected");
        fs::write(root.join("plain.txt"), b"same").unwrap();

        let files = collect_files(
            std::slice::from_ref(&root),
            &[root.join("does-not-exist")],
            true,
        )
        .unwrap();

        assert_eq!(files.files.len(), 1);
        assert!(!files.files[0].is_protected);
        assert!(files.errors.is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_missing_scan_roots() {
        let missing = std::env::temp_dir().join("dedupeforge-missing-root");
        let err = collect_files(&[missing.clone()], &[], true).unwrap_err();

        assert!(err.to_string().contains("scan root does not exist"));
    }
}
