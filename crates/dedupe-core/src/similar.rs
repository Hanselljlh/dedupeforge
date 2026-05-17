use crate::fs_walk::{collect_files, FileEntry};
use crate::scan::{DuplicateGroup, DuplicateItem, MatchRisk, ScanConfig, ScanMode, ScanReport};
use anyhow::Result;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn scan_similar_names(config: &ScanConfig) -> Result<ScanReport> {
    let collected = collect_files(&config.paths, &config.protected_roots, config.ignore_hidden)?;
    let files = collected
        .files
        .into_iter()
        .filter(|f| f.size >= config.min_size)
        .collect::<Vec<_>>();

    let duplicate_groups = group_similar_name_files(&files, config.name_similarity_threshold);

    Ok(ScanReport {
        mode: ScanMode::SimilarNames,
        scanned_files: files.len(),
        candidate_size_groups: 0,
        cache_hits: 0,
        cache_misses: 0,
        duplicate_groups,
        errors: collected.errors,
        risk: MatchRisk::High,
    })
}

pub fn scan_duplicate_folders(config: &ScanConfig) -> Result<ScanReport> {
    let directories = collect_directories(
        &config.paths,
        &config.protected_roots,
        config.ignore_hidden,
        &config.ignore_patterns,
    )?;
    let scanned_files = directories.values().map(|d| d.file_count).sum();
    let duplicate_groups =
        group_similar_directories(&directories, config.folder_similarity_threshold);

    Ok(ScanReport {
        mode: ScanMode::DuplicateFolders,
        scanned_files,
        candidate_size_groups: 0,
        cache_hits: 0,
        cache_misses: 0,
        duplicate_groups,
        errors: Vec::new(),
        risk: MatchRisk::Medium,
    })
}

fn group_similar_name_files(files: &[FileEntry], threshold: u8) -> Vec<DuplicateGroup> {
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); files.len()];
    let mut pair_reasons: HashMap<(usize, usize), (u8, String)> = HashMap::new();

    for left in 0..files.len() {
        for right in (left + 1)..files.len() {
            let left_name = file_stem_like(&files[left].path);
            let right_name = file_stem_like(&files[right].path);
            let (score, reason) = name_similarity_reason(&left_name, &right_name);
            if score >= threshold {
                adjacency[left].push(right);
                adjacency[right].push(left);
                pair_reasons.insert((left, right), (score, reason));
            }
        }
    }

    build_connected_groups(files, adjacency, pair_reasons, "similar name")
}

fn group_similar_directories(
    directories: &HashMap<PathBuf, DirectorySignature>,
    threshold: u8,
) -> Vec<DuplicateGroup> {
    let entries = directories
        .values()
        .filter(|d| !d.entries.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); entries.len()];
    let mut pair_reasons: HashMap<(usize, usize), (u8, String)> = HashMap::new();

    for left in 0..entries.len() {
        for right in (left + 1)..entries.len() {
            let (score, reason) = directory_similarity_reason(&entries[left], &entries[right]);
            if score >= threshold {
                adjacency[left].push(right);
                adjacency[right].push(left);
                pair_reasons.insert((left, right), (score, reason));
            }
        }
    }

    let as_files = entries
        .iter()
        .map(|entry| FileEntry {
            path: entry.path.clone(),
            size: entry.total_size,
            modified_unix: entry.modified_unix,
            identity: None,
            is_protected: entry.is_protected,
        })
        .collect::<Vec<_>>();

    build_connected_groups(&as_files, adjacency, pair_reasons, "similar folder")
}

fn build_connected_groups(
    items: &[FileEntry],
    adjacency: Vec<Vec<usize>>,
    pair_reasons: HashMap<(usize, usize), (u8, String)>,
    algorithm_label: &str,
) -> Vec<DuplicateGroup> {
    let mut visited = vec![false; items.len()];
    let mut groups = Vec::new();

    for start in 0..items.len() {
        if visited[start] || adjacency[start].is_empty() {
            continue;
        }

        let mut stack = vec![start];
        let mut component = Vec::new();
        visited[start] = true;

        while let Some(current) = stack.pop() {
            component.push(current);
            for &next in &adjacency[current] {
                if !visited[next] {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }

        if component.len() < 2 {
            continue;
        }

        component.sort_by_key(|&idx| item_sort_key(&items[idx]));
        let keep_index = choose_keep_index_for_indices(items, &component);
        let reason = best_component_reason(&component, &pair_reasons);
        let score = reason.0.to_string();
        let items = component
            .into_iter()
            .enumerate()
            .map(|(position, idx)| DuplicateItem {
                path: items[idx].path.clone(),
                size: items[idx].size,
                modified_unix: items[idx].modified_unix,
                is_protected: items[idx].is_protected,
                suggested_keep: position == keep_index,
            })
            .collect::<Vec<_>>();

        groups.push(DuplicateGroup {
            size: items.iter().map(|item| item.size).max().unwrap_or_default(),
            algorithm: algorithm_label.to_string(),
            hash: score,
            reason: reason.1,
            items,
        });
    }

    groups.sort_by(|a, b| {
        b.size
            .cmp(&a.size)
            .then_with(|| a.items[0].path.cmp(&b.items[0].path))
    });
    groups
}

fn best_component_reason(
    component: &[usize],
    pair_reasons: &HashMap<(usize, usize), (u8, String)>,
) -> (u8, String) {
    let mut best = (0u8, "matched by threshold".to_string());
    for left_index in 0..component.len() {
        for right_index in (left_index + 1)..component.len() {
            let left = component[left_index];
            let right = component[right_index];
            let key = if left < right {
                (left, right)
            } else {
                (right, left)
            };
            if let Some(reason) = pair_reasons.get(&key) {
                if reason.0 > best.0 {
                    best = reason.clone();
                }
            }
        }
    }
    best
}

fn choose_keep_index_for_indices(items: &[FileEntry], indices: &[usize]) -> usize {
    indices
        .iter()
        .position(|&idx| items[idx].is_protected)
        .unwrap_or(0)
}

fn item_sort_key(f: &FileEntry) -> (bool, Option<i64>, usize, String) {
    (
        !f.is_protected,
        f.modified_unix,
        f.path.components().count(),
        f.path.to_string_lossy().to_lowercase(),
    )
}

fn file_stem_like(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string()
}

fn name_similarity_reason(left: &str, right: &str) -> (u8, String) {
    let left_normalized = normalize_name(left);
    let right_normalized = normalize_name(right);

    if left_normalized == right_normalized {
        return (
            100,
            format!("normalized filename match ({left} ~= {right})"),
        );
    }

    let left_tokens = tokenize_name(&left_normalized);
    let right_tokens = tokenize_name(&right_normalized);
    let token_score = jaccard_score(&left_tokens, &right_tokens);
    let edit_score = edit_similarity_score(&left_normalized, &right_normalized);

    if token_score >= edit_score {
        (
            token_score,
            format!(
                "similar filename by token overlap {}% ({left} ~= {right})",
                token_score
            ),
        )
    } else {
        (
            edit_score,
            format!(
                "similar filename by edit distance {}% ({left} ~= {right})",
                edit_score
            ),
        )
    }
}

fn normalize_name(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_was_separator = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            normalized.push(' ');
            previous_was_separator = true;
        }
    }

    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn tokenize_name(value: &str) -> BTreeSet<String> {
    value
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect()
}

fn jaccard_score(left: &BTreeSet<String>, right: &BTreeSet<String>) -> u8 {
    if left.is_empty() && right.is_empty() {
        return 100;
    }

    let intersection = left.intersection(right).count();
    let union = left.union(right).count();
    ((intersection as f64 / union as f64) * 100.0).round() as u8
}

fn edit_similarity_score(left: &str, right: &str) -> u8 {
    if left.is_empty() && right.is_empty() {
        return 100;
    }
    let max_len = left.chars().count().max(right.chars().count()) as f64;
    let distance = levenshtein(left, right) as f64;
    (((max_len - distance).max(0.0) / max_len) * 100.0).round() as u8
}

fn levenshtein(left: &str, right: &str) -> usize {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0usize; right_chars.len() + 1];

    for (i, left_char) in left_chars.iter().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right_chars.iter().enumerate() {
            let cost = if left_char == right_char { 0 } else { 1 };
            current[j + 1] = (current[j] + 1)
                .min(previous[j + 1] + 1)
                .min(previous[j] + cost);
        }
        previous.clone_from(&current);
    }

    previous[right_chars.len()]
}

#[derive(Clone, Debug)]
struct DirectorySignature {
    path: PathBuf,
    entries: HashSet<String>,
    total_size: u64,
    file_count: usize,
    modified_unix: Option<i64>,
    is_protected: bool,
}

fn collect_directories(
    roots: &[PathBuf],
    protected_roots: &[PathBuf],
    ignore_hidden: bool,
    ignore_patterns: &[String],
) -> Result<HashMap<PathBuf, DirectorySignature>> {
    let protected_roots = protected_roots
        .iter()
        .map(|path| std::fs::canonicalize(path).unwrap_or_else(|_| path.clone()))
        .collect::<Vec<_>>();
    let mut directories = HashMap::<PathBuf, DirectorySignature>::new();

    for root in roots {
        let canonical_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.clone());
        for entry in WalkDir::new(&canonical_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| !ignore_hidden || !is_hidden_walk_entry(entry))
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() {
                continue;
            }

            let path =
                std::fs::canonicalize(entry.path()).unwrap_or_else(|_| entry.path().to_path_buf());
            let relative = path
                .strip_prefix(&canonical_root)
                .unwrap_or(path.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            if matches_ignore_pattern(&relative, ignore_patterns) {
                continue;
            }

            let parent = path.parent().unwrap_or(&canonical_root).to_path_buf();
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let modified_unix = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);
            let is_protected = protected_roots.iter().any(|p| parent.starts_with(p));

            let directory =
                directories
                    .entry(parent.clone())
                    .or_insert_with(|| DirectorySignature {
                        path: parent.clone(),
                        entries: HashSet::new(),
                        total_size: 0,
                        file_count: 0,
                        modified_unix,
                        is_protected,
                    });
            let entry_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&relative);
            directory
                .entries
                .insert(relative_file_signature(entry_name, metadata.len()));
            directory.total_size += metadata.len();
            directory.file_count += 1;
            directory.modified_unix = directory.modified_unix.max(modified_unix);
            directory.is_protected |= is_protected;
        }
    }

    Ok(directories)
}

fn relative_file_signature(relative: &str, size: u64) -> String {
    let normalized = relative
        .split('/')
        .map(normalize_name)
        .collect::<Vec<_>>()
        .join("/");
    format!("{normalized}:{size}")
}

fn directory_similarity_reason(
    left: &DirectorySignature,
    right: &DirectorySignature,
) -> (u8, String) {
    let score = jaccard_hashset_score(&left.entries, &right.entries);
    (
        score,
        format!(
            "similar folder by file-tree overlap {}% ({} files vs {} files)",
            score, left.file_count, right.file_count
        ),
    )
}

fn jaccard_hashset_score(left: &HashSet<String>, right: &HashSet<String>) -> u8 {
    if left.is_empty() && right.is_empty() {
        return 100;
    }

    let intersection = left.intersection(right).count();
    let union = left.union(right).count();
    ((intersection as f64 / union as f64) * 100.0).round() as u8
}

fn matches_ignore_pattern(relative: &str, patterns: &[String]) -> bool {
    patterns
        .iter()
        .any(|pattern| wildcard_match(relative, pattern))
}

fn wildcard_match(value: &str, pattern: &str) -> bool {
    wildcard_match_bytes(value.as_bytes(), pattern.as_bytes())
}

fn wildcard_match_bytes(value: &[u8], pattern: &[u8]) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }

    match pattern[0] {
        b'*' => {
            wildcard_match_bytes(value, &pattern[1..])
                || (!value.is_empty() && wildcard_match_bytes(&value[1..], pattern))
        }
        b'?' => !value.is_empty() && wildcard_match_bytes(&value[1..], &pattern[1..]),
        expected => {
            !value.is_empty()
                && value[0].eq_ignore_ascii_case(&expected)
                && wildcard_match_bytes(&value[1..], &pattern[1..])
        }
    }
}

fn is_hidden_walk_entry(entry: &walkdir::DirEntry) -> bool {
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
        let path = std::env::temp_dir().join(format!("dedupeforge-similar-{unique}-{name}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn base_config(root: &Path) -> ScanConfig {
        ScanConfig {
            mode: ScanMode::SimilarNames,
            paths: vec![root.to_path_buf()],
            protected_roots: vec![],
            algorithm: crate::HashAlgorithm::Blake3,
            partial_bytes: 1024,
            min_size: 1,
            ignore_hidden: true,
            byte_verify: false,
            cache: crate::scan::CacheConfig {
                enabled: false,
                path: None,
                modified_time_tolerance_secs: 0,
            },
            name_similarity_threshold: 70,
            folder_similarity_threshold: 70,
            ignore_patterns: Vec::new(),
        }
    }

    #[test]
    fn similar_name_scan_matches_normalized_names() {
        let root = temp_dir("names");
        fs::write(root.join("Vacation 2024.jpg"), b"a").unwrap();
        fs::write(root.join("vacation_2024 copy.jpg"), b"b").unwrap();
        fs::write(root.join("taxes.pdf"), b"c").unwrap();

        let report = scan_similar_names(&base_config(&root)).unwrap();

        assert_eq!(report.mode, ScanMode::SimilarNames);
        assert_eq!(report.risk, MatchRisk::High);
        assert_eq!(report.duplicate_groups.len(), 1);
        assert!(report.duplicate_groups[0].reason.contains("filename"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_folder_scan_respects_ignore_patterns() {
        let root = temp_dir("folders");
        let left = root.join("left");
        let right = root.join("right");
        fs::create_dir_all(&left).unwrap();
        fs::create_dir_all(&right).unwrap();
        fs::write(left.join("photo.jpg"), b"same").unwrap();
        fs::write(right.join("photo.jpg"), b"diff").unwrap();
        fs::write(left.join("skip.tmp"), b"noise").unwrap();
        fs::write(right.join("skip.tmp"), b"other-noise").unwrap();

        let mut config = base_config(&root);
        config.mode = ScanMode::DuplicateFolders;
        config.ignore_patterns = vec!["*.tmp".to_string()];

        let report = scan_duplicate_folders(&config).unwrap();

        assert_eq!(report.mode, ScanMode::DuplicateFolders);
        assert_eq!(report.duplicate_groups.len(), 1);
        assert!(report.duplicate_groups[0]
            .reason
            .contains("file-tree overlap"));

        fs::remove_dir_all(root).unwrap();
    }
}
