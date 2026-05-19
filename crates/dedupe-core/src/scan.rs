use crate::fs_walk::{collect_files, FileEntry, FileIdentity};
use crate::hash::{hash_bytes, hash_file, hash_file_prefix, HashAlgorithm};
use crate::similar::{
    scan_duplicate_folders, scan_empty_files, scan_empty_folders, scan_raw_jpeg_pairs,
    scan_similar_images, scan_similar_names,
};
use crate::verify::files_equal;
use anyhow::Result;
use dedupe_cache::{Cache, CacheFileIdentity, CacheHashKey, CacheLookupPolicy, HashScope};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanConfig {
    pub mode: ScanMode,
    pub paths: Vec<PathBuf>,
    pub protected_roots: Vec<PathBuf>,
    pub algorithm: HashAlgorithm,
    pub partial_bytes: u64,
    pub min_size: u64,
    pub ignore_hidden: bool,
    pub byte_verify: bool,
    pub cache: CacheConfig,
    pub name_similarity_threshold: u8,
    pub folder_similarity_threshold: u8,
    pub image_hash_size: u32,
    pub image_hamming_threshold: u32,
    pub image_rotation_invariant: bool,
    pub media_duration_tolerance_secs: f64,
    pub media_fingerprint_distance_threshold: u32,
    pub scan_archives: bool,
    pub ignore_patterns: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub path: Option<PathBuf>,
    pub modified_time_tolerance_secs: i64,
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
    pub mode: ScanMode,
    pub scanned_files: usize,
    pub candidate_size_groups: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub errors: Vec<String>,
    pub risk: MatchRisk,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanProgress {
    pub phase: ScanProgressPhase,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

#[derive(Clone, Default)]
pub struct ScanCancelToken {
    cancelled: Arc<AtomicBool>,
}

impl ScanCancelToken {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ScanProgressPhase {
    CollectingFiles,
    GroupingBySize,
    PartialHashing,
    FullHashing,
    ByteVerifying,
    ScanningArchives,
    BuildingResults,
    Finished,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ScanMode {
    Exact,
    SimilarNames,
    SimilarImages,
    RawJpegPairs,
    SimilarVideos,
    SimilarAudio,
    DuplicateFolders,
    EmptyFiles,
    EmptyFolders,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MatchRisk {
    Low,
    Medium,
    High,
}

pub fn scan(config: &ScanConfig) -> Result<ScanReport> {
    scan_with_progress(config, ScanCancelToken::default(), |_| {})
}

pub fn scan_with_progress<F>(
    config: &ScanConfig,
    cancel: ScanCancelToken,
    mut on_progress: F,
) -> Result<ScanReport>
where
    F: FnMut(ScanProgress),
{
    ensure_not_cancelled(&cancel)?;
    match config.mode {
        ScanMode::Exact => scan_exact_with_progress(config, &cancel, &mut on_progress),
        ScanMode::SimilarNames => {
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::CollectingFiles,
                0,
                0,
                "Scanning similar filenames".to_string(),
            ));
            let report = scan_similar_names(config)?;
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::Finished,
                report.scanned_files,
                report.scanned_files,
                format!("Scan complete: {} groups", report.duplicate_groups.len()),
            ));
            Ok(report)
        }
        ScanMode::SimilarImages => {
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::CollectingFiles,
                0,
                0,
                "Scanning similar images".to_string(),
            ));
            let report = scan_similar_images(config)?;
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::Finished,
                report.scanned_files,
                report.scanned_files,
                format!("Scan complete: {} groups", report.duplicate_groups.len()),
            ));
            Ok(report)
        }
        ScanMode::RawJpegPairs => {
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::CollectingFiles,
                0,
                0,
                "Scanning RAW + JPEG pairs".to_string(),
            ));
            let report = scan_raw_jpeg_pairs(config)?;
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::Finished,
                report.scanned_files,
                report.scanned_files,
                format!("Scan complete: {} groups", report.duplicate_groups.len()),
            ));
            Ok(report)
        }
        ScanMode::SimilarVideos => {
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::CollectingFiles,
                0,
                0,
                "Scanning similar videos".to_string(),
            ));
            let report = crate::similar::scan_similar_videos(config)?;
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::Finished,
                report.scanned_files,
                report.scanned_files,
                format!("Scan complete: {} groups", report.duplicate_groups.len()),
            ));
            Ok(report)
        }
        ScanMode::SimilarAudio => {
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::CollectingFiles,
                0,
                0,
                "Scanning similar audio".to_string(),
            ));
            let report = crate::similar::scan_similar_audio(config)?;
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::Finished,
                report.scanned_files,
                report.scanned_files,
                format!("Scan complete: {} groups", report.duplicate_groups.len()),
            ));
            Ok(report)
        }
        ScanMode::DuplicateFolders => {
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::CollectingFiles,
                0,
                0,
                "Scanning duplicate folders".to_string(),
            ));
            let report = scan_duplicate_folders(config)?;
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::Finished,
                report.scanned_files,
                report.scanned_files,
                format!("Scan complete: {} groups", report.duplicate_groups.len()),
            ));
            Ok(report)
        }
        ScanMode::EmptyFiles => {
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::CollectingFiles,
                0,
                0,
                "Scanning empty files".to_string(),
            ));
            let report = scan_empty_files(config)?;
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::Finished,
                report.scanned_files,
                report.scanned_files,
                format!("Scan complete: {} groups", report.duplicate_groups.len()),
            ));
            Ok(report)
        }
        ScanMode::EmptyFolders => {
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::CollectingFiles,
                0,
                0,
                "Scanning empty folders".to_string(),
            ));
            let report = scan_empty_folders(config)?;
            ensure_not_cancelled(&cancel)?;
            on_progress(progress_event(
                ScanProgressPhase::Finished,
                report.scanned_files,
                report.scanned_files,
                format!("Scan complete: {} groups", report.duplicate_groups.len()),
            ));
            Ok(report)
        }
    }
}

pub fn scan_exact(config: &ScanConfig) -> Result<ScanReport> {
    scan_exact_with_progress(config, &ScanCancelToken::default(), &mut |_| {})
}

pub fn scan_exact_with_progress<F>(
    config: &ScanConfig,
    cancel: &ScanCancelToken,
    on_progress: &mut F,
) -> Result<ScanReport>
where
    F: FnMut(ScanProgress),
{
    ensure_not_cancelled(cancel)?;
    on_progress(progress_event(
        ScanProgressPhase::CollectingFiles,
        0,
        config.paths.len(),
        "Collecting files".to_string(),
    ));
    let cache = open_cache_if_enabled(&config.cache)?;
    let collected = collect_files(&config.paths, &config.protected_roots, config.ignore_hidden)?;
    ensure_not_cancelled(cancel)?;
    let files = collected
        .files
        .into_iter()
        .filter(|f| f.size >= config.min_size)
        .collect::<Vec<_>>();

    let scanned_files = files.len();
    on_progress(progress_event(
        ScanProgressPhase::GroupingBySize,
        scanned_files,
        scanned_files,
        format!("Grouping {scanned_files} files by size"),
    ));
    let by_size = group_by_size(files);
    ensure_not_cancelled(cancel)?;
    let candidate_size_groups = by_size.values().filter(|g| g.len() > 1).count();

    let mut stats = HashRunStats {
        errors: collected.errors,
        ..Default::default()
    };
    let size_candidate_files: Vec<FileEntry> = by_size.into_values().flatten().collect();
    on_progress(progress_event(
        ScanProgressPhase::PartialHashing,
        0,
        size_candidate_files.len(),
        format!("Partial hashing {} candidates", size_candidate_files.len()),
    ));
    let partial_candidates = hash_files(
        cache.as_ref(),
        size_candidate_files,
        HashRunOptions {
            algorithm: config.algorithm,
            partial_bytes: config.partial_bytes,
            partial: true,
            modified_time_tolerance_secs: config.cache.modified_time_tolerance_secs,
        },
        &mut stats,
        on_progress,
        ScanProgressPhase::PartialHashing,
        cancel,
    );
    let partial_candidate_files: Vec<FileEntry> =
        partial_candidates.into_values().flatten().collect();
    on_progress(progress_event(
        ScanProgressPhase::FullHashing,
        0,
        partial_candidate_files.len(),
        format!("Full hashing {} candidates", partial_candidate_files.len()),
    ));
    let full_candidates = hash_files(
        cache.as_ref(),
        partial_candidate_files,
        HashRunOptions {
            algorithm: config.algorithm,
            partial_bytes: 0,
            partial: false,
            modified_time_tolerance_secs: config.cache.modified_time_tolerance_secs,
        },
        &mut stats,
        on_progress,
        ScanProgressPhase::FullHashing,
        cancel,
    );

    let mut duplicate_groups = Vec::new();
    let full_group_count = full_candidates.len();
    if config.byte_verify {
        on_progress(progress_event(
            ScanProgressPhase::ByteVerifying,
            0,
            full_group_count,
            format!("Byte verifying {full_group_count} groups"),
        ));
    } else {
        on_progress(progress_event(
            ScanProgressPhase::BuildingResults,
            0,
            full_group_count,
            format!("Building {full_group_count} result groups"),
        ));
    }

    for (group_index, ((_size, full_hash), mut group)) in full_candidates.into_iter().enumerate() {
        ensure_not_cancelled(cancel)?;
        if group.len() < 2 {
            continue;
        }
        group.sort_by_key(item_sort_key);

        if config.byte_verify {
            on_progress(progress_event(
                ScanProgressPhase::ByteVerifying,
                group_index + 1,
                full_group_count,
                format!(
                    "Byte verifying group {} of {}",
                    group_index + 1,
                    full_group_count
                ),
            ));
            for verified in split_by_byte_equality(&group, &mut stats.errors) {
                if verified.len() > 1 {
                    duplicate_groups.push(make_group(
                        verified,
                        config.algorithm,
                        full_hash.clone(),
                        true,
                    ));
                }
            }
        } else {
            on_progress(progress_event(
                ScanProgressPhase::BuildingResults,
                group_index + 1,
                full_group_count,
                format!("Building group {} of {}", group_index + 1, full_group_count),
            ));
            duplicate_groups.push(make_group(group, config.algorithm, full_hash, false));
        }
    }

    duplicate_groups.sort_by(|a, b| {
        b.size
            .cmp(&a.size)
            .then_with(|| a.items[0].path.cmp(&b.items[0].path))
    });
    if config.scan_archives {
        ensure_not_cancelled(cancel)?;
        on_progress(progress_event(
            ScanProgressPhase::ScanningArchives,
            0,
            config.paths.len(),
            "Scanning zip archives".to_string(),
        ));
        duplicate_groups.extend(scan_zip_archives_exact(config, &mut stats.errors)?);
        duplicate_groups.sort_by(|a, b| {
            b.size
                .cmp(&a.size)
                .then_with(|| a.items[0].path.cmp(&b.items[0].path))
        });
    }

    let report = ScanReport {
        mode: ScanMode::Exact,
        scanned_files,
        candidate_size_groups,
        cache_hits: stats.cache_hits,
        cache_misses: stats.cache_misses,
        duplicate_groups,
        errors: stats.errors,
        risk: MatchRisk::Low,
    };
    on_progress(progress_event(
        ScanProgressPhase::Finished,
        report.scanned_files,
        report.scanned_files,
        format!("Scan complete: {} groups", report.duplicate_groups.len()),
    ));
    Ok(report)
}

fn group_by_size(files: Vec<FileEntry>) -> HashMap<u64, Vec<FileEntry>> {
    let mut map: HashMap<u64, Vec<FileEntry>> = HashMap::new();
    for file in files {
        map.entry(file.size).or_default().push(file);
    }
    map.retain(|_, group| group.len() > 1);
    map
}

#[derive(Clone, Copy)]
struct HashRunOptions {
    algorithm: HashAlgorithm,
    partial_bytes: u64,
    partial: bool,
    modified_time_tolerance_secs: i64,
}

#[derive(Default)]
struct HashRunStats {
    errors: Vec<String>,
    cache_hits: usize,
    cache_misses: usize,
}

type HashOutcome = (FileEntry, Result<(String, bool), String>);

struct PendingHash {
    file: FileEntry,
    cache_key: CacheHashKeyOwned,
}

#[derive(Clone)]
struct CacheHashKeyOwned {
    path: PathBuf,
    identity: Option<CacheFileIdentity>,
    size: u64,
    modified_unix: Option<i64>,
    algorithm: &'static str,
    scope: HashScope,
}

impl CacheHashKeyOwned {
    fn as_borrowed(&self) -> CacheHashKey<'_> {
        CacheHashKey {
            path: &self.path,
            identity: self.identity.as_ref(),
            size: self.size,
            modified_unix: self.modified_unix,
            algorithm: self.algorithm,
            scope: self.scope,
        }
    }
}

fn hash_files(
    cache: Option<&Cache>,
    files: Vec<FileEntry>,
    options: HashRunOptions,
    stats: &mut HashRunStats,
    on_progress: &mut impl FnMut(ScanProgress),
    phase: ScanProgressPhase,
    cancel: &ScanCancelToken,
) -> HashMap<(u64, String), Vec<FileEntry>> {
    let total = files.len();
    let hashed: Vec<HashOutcome> = if let Some(cache) = cache {
        let mut completed = Vec::with_capacity(total);
        let mut pending = Vec::new();

        for file in files {
            if cancel.is_cancelled() {
                break;
            }
            let cache_key = cache_hash_key(
                &file,
                options.algorithm,
                options.partial_bytes,
                options.partial,
            );
            match cache
                .lookup_hash(
                    &cache_key.as_borrowed(),
                    CacheLookupPolicy {
                        modified_time_tolerance_secs: options.modified_time_tolerance_secs,
                    },
                )
                .and_then(|found| {
                    if found.is_some() {
                        cache.mark_seen(&file.path)?;
                    }
                    Ok(found)
                }) {
                Ok(Some(found)) => completed.push((file, Ok((found.hash, true)))),
                Ok(None) => pending.push(PendingHash { file, cache_key }),
                Err(e) => completed.push((file, Err(e.to_string()))),
            }
        }

        let mut hashed_misses: Vec<(PendingHash, Result<String, String>)> = pending
            .into_par_iter()
            .map(|pending| {
                let hash_result = hash_without_cache(
                    &pending.file,
                    options.algorithm,
                    options.partial_bytes,
                    options.partial,
                )
                .map(|(hash, _)| hash)
                .map_err(|e| e.to_string());
                (pending, hash_result)
            })
            .collect();

        for (pending, result) in hashed_misses.drain(..) {
            match result {
                Ok(hash) => {
                    if let Err(e) = cache.store_hash(&pending.cache_key.as_borrowed(), &hash) {
                        completed.push((
                            pending.file,
                            Err(format!("failed to store cache entry: {e}")),
                        ));
                    } else {
                        completed.push((pending.file, Ok((hash, false))));
                    }
                }
                Err(error) => completed.push((pending.file, Err(error))),
            }
        }

        completed
    } else {
        files
            .into_par_iter()
            .map(|file| {
                let hash_result = hash_without_cache(
                    &file,
                    options.algorithm,
                    options.partial_bytes,
                    options.partial,
                );
                match hash_result {
                    Ok((hash, used_cache)) => (file, Ok((hash, used_cache))),
                    Err(e) => (file, Err(e.to_string())),
                }
            })
            .collect()
    };

    let mut map: HashMap<(u64, String), Vec<FileEntry>> = HashMap::new();

    for (index, (file, maybe_hash)) in hashed.into_iter().enumerate() {
        if cancel.is_cancelled() {
            break;
        }
        match maybe_hash {
            Err(error) => stats
                .errors
                .push(format!("{}: {error}", file.path.display())),
            Ok((hash, used_cache)) => {
                if used_cache {
                    stats.cache_hits += 1;
                } else {
                    stats.cache_misses += 1;
                }
                map.entry((file.size, hash)).or_default().push(file);
            }
        }
        on_progress(progress_event(
            phase,
            index + 1,
            total,
            progress_message(phase, index + 1, total),
        ));
    }

    map.retain(|_, group| group.len() > 1);
    map
}

fn ensure_not_cancelled(cancel: &ScanCancelToken) -> Result<()> {
    if cancel.is_cancelled() {
        anyhow::bail!("scan canceled");
    }
    Ok(())
}

fn progress_event(
    phase: ScanProgressPhase,
    current: usize,
    total: usize,
    message: String,
) -> ScanProgress {
    ScanProgress {
        phase,
        current,
        total,
        message,
    }
}

fn progress_message(phase: ScanProgressPhase, current: usize, total: usize) -> String {
    let label = match phase {
        ScanProgressPhase::CollectingFiles => "Collecting files",
        ScanProgressPhase::GroupingBySize => "Grouping files",
        ScanProgressPhase::PartialHashing => "Partial hashing",
        ScanProgressPhase::FullHashing => "Full hashing",
        ScanProgressPhase::ByteVerifying => "Byte verifying",
        ScanProgressPhase::ScanningArchives => "Scanning archives",
        ScanProgressPhase::BuildingResults => "Building results",
        ScanProgressPhase::Finished => "Finished",
    };
    if total == 0 {
        label.to_string()
    } else {
        format!("{label}: {current}/{total}")
    }
}

fn hash_without_cache(
    file: &FileEntry,
    algorithm: HashAlgorithm,
    partial_bytes: u64,
    partial: bool,
) -> Result<(String, bool)> {
    let hash = if partial {
        hash_file_prefix(&file.path, algorithm, partial_bytes)?
    } else {
        hash_file(&file.path, algorithm)?
    };
    Ok((hash, false))
}

fn cache_hash_key(
    file: &FileEntry,
    algorithm: HashAlgorithm,
    partial_bytes: u64,
    partial: bool,
) -> CacheHashKeyOwned {
    let scope = if partial {
        HashScope::Partial {
            bytes_hashed: partial_bytes,
        }
    } else {
        HashScope::Full
    };

    CacheHashKeyOwned {
        path: file.path.clone(),
        identity: cache_identity(file),
        size: file.size,
        modified_unix: file.modified_unix,
        algorithm: algorithm.label(),
        scope,
    }
}

fn scan_zip_archives_exact(
    config: &ScanConfig,
    errors: &mut Vec<String>,
) -> Result<Vec<DuplicateGroup>> {
    let mut by_hash: HashMap<(u64, String), Vec<DuplicateItem>> = HashMap::new();

    for root in &config.paths {
        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !matches!(
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.to_ascii_lowercase())
                    .as_deref(),
                Some("zip")
            ) {
                continue;
            }

            if let Err(err) = collect_zip_members(path, config, &mut by_hash) {
                errors.push(format!("{}: {err}", path.display()));
            }
        }
    }

    let mut groups = Vec::new();
    for ((size, hash), mut items) in by_hash {
        if items.len() < 2 {
            continue;
        }
        items.sort_by(|a, b| a.path.cmp(&b.path));
        if let Some(first) = items.first_mut() {
            first.suggested_keep = true;
        }
        groups.push(DuplicateGroup {
            size,
            algorithm: format!("{} (archive)", config.algorithm.label()),
            hash,
            reason: "same archive member size + same full hash".to_string(),
            items,
        });
    }

    Ok(groups)
}

fn collect_zip_members(
    archive_path: &Path,
    config: &ScanConfig,
    by_hash: &mut HashMap<(u64, String), Vec<DuplicateItem>>,
) -> Result<()> {
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for index in 0..archive.len() {
        let mut member = archive.by_index(index)?;
        if !member.is_file() {
            continue;
        }
        let size = member.size();
        if size < config.min_size {
            continue;
        }
        let mut bytes = Vec::new();
        use std::io::Read;
        member.read_to_end(&mut bytes)?;
        let hash = hash_bytes(&bytes, config.algorithm)?;
        let pseudo_path = PathBuf::from(format!("{}!{}", archive_path.display(), member.name()));
        by_hash
            .entry((size, hash.clone()))
            .or_default()
            .push(DuplicateItem {
                path: pseudo_path,
                size,
                modified_unix: None,
                is_protected: true,
                suggested_keep: false,
            });
    }
    Ok(())
}

fn cache_identity(file: &FileEntry) -> Option<CacheFileIdentity> {
    file.identity
        .as_ref()
        .map(cache_identity_from_file_identity)
}

fn cache_identity_from_file_identity(identity: &FileIdentity) -> CacheFileIdentity {
    CacheFileIdentity {
        device_id: identity.device_id.clone(),
        inode: identity.inode.clone(),
    }
}

fn open_cache_if_enabled(config: &CacheConfig) -> Result<Option<Cache>> {
    if !config.enabled {
        return Ok(None);
    }

    let path = config
        .path
        .as_deref()
        .unwrap_or_else(|| Path::new(".dedupeforge-cache.sqlite3"));
    Ok(Some(Cache::open(path)?))
}

fn split_by_byte_equality(group: &[FileEntry], errors: &mut Vec<String>) -> Vec<Vec<FileEntry>> {
    let mut verified_groups: Vec<Vec<FileEntry>> = Vec::new();

    'outer: for item in group.iter().cloned() {
        for existing_group in verified_groups.iter_mut() {
            let representative = &existing_group[0];
            match files_equal(&representative.path, &item.path) {
                Ok(true) => {
                    existing_group.push(item);
                    continue 'outer;
                }
                Ok(false) => {}
                Err(e) => errors.push(format!(
                    "byte verify failed for {}: {e}",
                    item.path.display()
                )),
            }
        }
        verified_groups.push(vec![item]);
    }

    verified_groups
}

fn make_group(
    mut files: Vec<FileEntry>,
    algorithm: HashAlgorithm,
    hash: String,
    byte_verified: bool,
) -> DuplicateGroup {
    files.sort_by_key(item_sort_key);
    let keep_index = choose_keep_index(&files);
    let items = files
        .into_iter()
        .enumerate()
        .map(|(idx, f)| DuplicateItem {
            path: f.path,
            size: f.size,
            modified_unix: f.modified_unix,
            is_protected: f.is_protected,
            suggested_keep: idx == keep_index,
        })
        .collect::<Vec<_>>();

    DuplicateGroup {
        size: items_first_size(&items),
        algorithm: algorithm.label().to_string(),
        hash,
        reason: if byte_verified {
            "same size + same full hash + byte-by-byte verified".to_string()
        } else {
            "same size + same full hash".to_string()
        },
        items,
    }
}

fn items_first_size(items: &[DuplicateItem]) -> u64 {
    items.first().map(|i| i.size).unwrap_or_default()
}

fn choose_keep_index(files: &[FileEntry]) -> usize {
    files.iter().position(|f| f.is_protected).unwrap_or(0)
}

fn item_sort_key(f: &FileEntry) -> (bool, Option<i64>, usize, String) {
    (
        !f.is_protected,
        f.modified_unix,
        f.path.components().count(),
        f.path.to_string_lossy().to_lowercase(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("dedupeforge-scan-{unique}-{name}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn exact_scan_reports_duplicate_group_and_keep_candidate() {
        let root = temp_dir("scan-basic");
        let archive = root.join("archive");
        let current = root.join("current");
        fs::create_dir_all(&archive).unwrap();
        fs::create_dir_all(&current).unwrap();

        fs::write(archive.join("photo.jpg"), b"same-image-bytes").unwrap();
        fs::write(current.join("photo-copy.jpg"), b"same-image-bytes").unwrap();
        fs::write(current.join("unique.jpg"), b"unique-content!").unwrap();

        let config = ScanConfig {
            mode: ScanMode::Exact,
            paths: vec![root.clone()],
            protected_roots: vec![archive.clone()],
            algorithm: HashAlgorithm::Blake3,
            partial_bytes: 4,
            min_size: 1,
            ignore_hidden: true,
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
        };

        let report = scan_exact(&config).unwrap();

        assert_eq!(report.scanned_files, 3);
        assert_eq!(report.candidate_size_groups, 1);
        assert_eq!(report.cache_hits, 0);
        assert_eq!(report.duplicate_groups.len(), 1);
        assert!(report.errors.is_empty());

        let group = &report.duplicate_groups[0];
        assert_eq!(group.reason, "same size + same full hash");
        assert_eq!(group.items.len(), 2);
        assert_eq!(group.items.iter().filter(|i| i.suggested_keep).count(), 1);
        assert!(group
            .items
            .iter()
            .any(|i| i.is_protected && i.suggested_keep));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn byte_verification_reason_is_reported_when_enabled() {
        let root = temp_dir("scan-verify");
        fs::write(root.join("a.bin"), b"verified-content").unwrap();
        fs::write(root.join("b.bin"), b"verified-content").unwrap();

        let config = ScanConfig {
            mode: ScanMode::Exact,
            paths: vec![root.clone()],
            protected_roots: vec![],
            algorithm: HashAlgorithm::Sha256,
            partial_bytes: 8,
            min_size: 1,
            ignore_hidden: true,
            byte_verify: true,
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
        };

        let report = scan_exact(&config).unwrap();

        assert_eq!(report.duplicate_groups.len(), 1);
        assert_eq!(
            report.duplicate_groups[0].reason,
            "same size + same full hash + byte-by-byte verified"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn second_scan_reuses_cached_hashes() {
        let root = temp_dir("scan-cache");
        let cache_path = root.join("cache.sqlite3");
        fs::write(root.join("a.bin"), b"same-bytes").unwrap();
        fs::write(root.join("b.bin"), b"same-bytes").unwrap();

        let config = ScanConfig {
            mode: ScanMode::Exact,
            paths: vec![root.clone()],
            protected_roots: vec![],
            algorithm: HashAlgorithm::Blake3,
            partial_bytes: 4,
            min_size: 1,
            ignore_hidden: true,
            byte_verify: false,
            cache: CacheConfig {
                enabled: true,
                path: Some(cache_path),
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
        };

        let first = scan_exact(&config).unwrap();
        let second = scan_exact(&config).unwrap();

        assert_eq!(first.cache_hits, 0);
        assert!(second.cache_hits >= 2);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scan_continues_when_hashing_one_candidate_fails() {
        let root = temp_dir("hash-error");
        let archive = root.join("archive");
        let current = root.join("current");
        fs::create_dir_all(&archive).unwrap();
        fs::create_dir_all(&current).unwrap();

        let keep = archive.join("keep.txt");
        let copy = current.join("copy.txt");
        let missing = current.join("missing.txt");

        fs::write(&keep, b"same").unwrap();
        fs::write(&copy, b"same").unwrap();
        fs::write(&missing, b"same").unwrap();

        let mut candidate_files = vec![
            FileEntry {
                path: keep.clone(),
                size: 4,
                modified_unix: Some(1),
                identity: None,
                is_protected: true,
            },
            FileEntry {
                path: copy.clone(),
                size: 4,
                modified_unix: Some(2),
                identity: None,
                is_protected: false,
            },
            FileEntry {
                path: missing.clone(),
                size: 4,
                modified_unix: Some(3),
                identity: None,
                is_protected: false,
            },
        ];

        fs::remove_file(&missing).unwrap();

        let mut stats = HashRunStats::default();
        let mut on_progress = |_| {};
        let cancel = ScanCancelToken::default();
        let hashed = hash_files(
            None,
            std::mem::take(&mut candidate_files),
            HashRunOptions {
                algorithm: HashAlgorithm::Blake3,
                partial_bytes: 0,
                partial: false,
                modified_time_tolerance_secs: 0,
            },
            &mut stats,
            &mut on_progress,
            ScanProgressPhase::FullHashing,
            &cancel,
        );

        assert_eq!(stats.errors.len(), 1);
        assert!(stats.errors[0].contains("missing.txt"));
        assert_eq!(hashed.len(), 1);
        assert_eq!(hashed.values().next().unwrap().len(), 2);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cache_reuses_hashes_when_modified_time_is_within_tolerance() {
        let root = temp_dir("scan-cache-tolerance");
        let cache_path = root.join("cache.sqlite3");
        let first = root.join("first.bin");
        let second = root.join("second.bin");
        fs::write(&first, b"same-bytes").unwrap();
        fs::write(&second, b"same-bytes").unwrap();

        let base = ScanConfig {
            mode: ScanMode::Exact,
            paths: vec![root.clone()],
            protected_roots: vec![],
            algorithm: HashAlgorithm::Blake3,
            partial_bytes: 4,
            min_size: 1,
            ignore_hidden: true,
            byte_verify: false,
            cache: CacheConfig {
                enabled: true,
                path: Some(cache_path.clone()),
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
        };

        let _ = scan_exact(&base).unwrap();

        let cache = Cache::open(&cache_path).unwrap();
        cache
            .store_hash(
                &CacheHashKey {
                    path: &first,
                    identity: None,
                    size: 10,
                    modified_unix: Some(100),
                    algorithm: "blake3",
                    scope: HashScope::Partial { bytes_hashed: 4 },
                },
                "synthetic-partial",
            )
            .unwrap();
        cache
            .store_hash(
                &CacheHashKey {
                    path: &second,
                    identity: None,
                    size: 10,
                    modified_unix: Some(100),
                    algorithm: "blake3",
                    scope: HashScope::Partial { bytes_hashed: 4 },
                },
                "synthetic-partial",
            )
            .unwrap();
        cache
            .store_hash(
                &CacheHashKey {
                    path: &first,
                    identity: None,
                    size: 10,
                    modified_unix: Some(100),
                    algorithm: "blake3",
                    scope: HashScope::Full,
                },
                "synthetic-full",
            )
            .unwrap();
        cache
            .store_hash(
                &CacheHashKey {
                    path: &second,
                    identity: None,
                    size: 10,
                    modified_unix: Some(100),
                    algorithm: "blake3",
                    scope: HashScope::Full,
                },
                "synthetic-full",
            )
            .unwrap();

        let candidate_files = vec![
            FileEntry {
                path: first,
                size: 10,
                modified_unix: Some(102),
                identity: None,
                is_protected: false,
            },
            FileEntry {
                path: second,
                size: 10,
                modified_unix: Some(102),
                identity: None,
                is_protected: false,
            },
        ];

        let mut stats = HashRunStats::default();
        let mut on_progress = |_| {};
        let cancel = ScanCancelToken::default();
        let hashed = hash_files(
            Some(&cache),
            candidate_files,
            HashRunOptions {
                algorithm: HashAlgorithm::Blake3,
                partial_bytes: 4,
                partial: true,
                modified_time_tolerance_secs: 2,
            },
            &mut stats,
            &mut on_progress,
            ScanProgressPhase::PartialHashing,
            &cancel,
        );

        assert!(stats.errors.is_empty());
        assert_eq!(stats.cache_hits, 2);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(hashed.len(), 1);

        drop(cache);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cache_reuses_hashes_when_path_changes_but_identity_matches() {
        let cache_path = temp_dir("scan-cache-rename").join("cache.sqlite3");
        let original = std::env::temp_dir().join("dedupeforge-rename-original.bin");
        let renamed = std::env::temp_dir().join("dedupeforge-rename-renamed.bin");
        fs::write(&original, b"same-bytes").unwrap();

        let cache = Cache::open(&cache_path).unwrap();
        let identity = CacheFileIdentity {
            device_id: "device-a".to_string(),
            inode: "inode-b".to_string(),
        };

        cache
            .store_hash(
                &CacheHashKey {
                    path: &original,
                    identity: Some(&identity),
                    size: 10,
                    modified_unix: Some(100),
                    algorithm: "blake3",
                    scope: HashScope::Full,
                },
                "identity-full",
            )
            .unwrap();

        let renamed_entry = FileEntry {
            path: renamed,
            size: 10,
            modified_unix: Some(100),
            identity: Some(FileIdentity {
                device_id: "device-a".to_string(),
                inode: "inode-b".to_string(),
            }),
            is_protected: false,
        };

        let mut stats = HashRunStats::default();
        let mut on_progress = |_| {};
        let cancel = ScanCancelToken::default();
        let hashed = hash_files(
            Some(&cache),
            vec![renamed_entry.clone(), renamed_entry],
            HashRunOptions {
                algorithm: HashAlgorithm::Blake3,
                partial_bytes: 0,
                partial: false,
                modified_time_tolerance_secs: 0,
            },
            &mut stats,
            &mut on_progress,
            ScanProgressPhase::FullHashing,
            &cancel,
        );

        assert!(stats.errors.is_empty());
        assert_eq!(stats.cache_hits, 2);
        assert_eq!(hashed.len(), 1);

        drop(cache);
        let _ = fs::remove_file(original);
        let _ = fs::remove_dir_all(cache_path.parent().unwrap());
    }

    #[test]
    fn exact_scan_can_report_duplicate_zip_members() {
        let root = temp_dir("archive-scan");
        let left_zip = root.join("left.zip");
        let right_zip = root.join("right.zip");

        write_zip(&left_zip, "a.txt", b"same-bytes");
        write_zip(&right_zip, "b.txt", b"same-bytes");

        let config = ScanConfig {
            mode: ScanMode::Exact,
            paths: vec![root.clone()],
            protected_roots: Vec::new(),
            algorithm: HashAlgorithm::Blake3,
            partial_bytes: 4,
            min_size: 1,
            ignore_hidden: true,
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
            scan_archives: true,
            ignore_patterns: Vec::new(),
        };

        let report = scan_exact(&config).unwrap();
        assert!(report
            .duplicate_groups
            .iter()
            .any(|group| group.algorithm.contains("archive")));
        assert!(report
            .duplicate_groups
            .iter()
            .flat_map(|group| group.items.iter())
            .any(|item| item.is_protected));

        let _ = fs::remove_dir_all(root);
    }

    fn write_zip(path: &Path, member_name: &str, bytes: &[u8]) {
        let file = fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file(member_name, options).unwrap();
        zip.write_all(bytes).unwrap();
        zip.finish().unwrap();
    }
}
