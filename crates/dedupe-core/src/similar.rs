use crate::fs_walk::{collect_files, FileEntry};
use crate::scan::{DuplicateGroup, DuplicateItem, MatchRisk, ScanConfig, ScanMode, ScanReport};
use anyhow::Result;
use dedupe_cache::{Cache, CacheHashKey, CacheLookupPolicy, HashScope};
use dedupe_media::{
    analyze_audio, analyze_image, analyze_video, compare_hashes_hex, media_tools_available,
    probe_metadata, read_exif_date, supported_audio_extension, supported_image_extension,
    supported_raw_extension, supported_video_extension, MediaToolConfig,
};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
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

pub fn scan_similar_images(config: &ScanConfig) -> Result<ScanReport> {
    let collected = collect_files(&config.paths, &config.protected_roots, config.ignore_hidden)?;
    let cache = open_cache_if_enabled(&config.cache)?;
    let files = collected
        .files
        .into_iter()
        .filter(|f| f.size >= config.min_size)
        .filter(|f| supported_image_extension(&f.path) || supported_raw_extension(&f.path))
        .collect::<Vec<_>>();

    let scanned_files = files.len();
    let mut cache_hits = 0usize;
    let mut cache_misses = 0usize;
    let mut errors = collected.errors;
    let duplicate_groups = group_similar_image_files(
        cache.as_ref(),
        &files,
        config,
        &mut errors,
        &mut cache_hits,
        &mut cache_misses,
    );

    Ok(ScanReport {
        mode: ScanMode::SimilarImages,
        scanned_files,
        candidate_size_groups: 0,
        cache_hits,
        cache_misses,
        duplicate_groups,
        errors,
        risk: MatchRisk::High,
    })
}

pub fn scan_raw_jpeg_pairs(config: &ScanConfig) -> Result<ScanReport> {
    let collected = collect_files(&config.paths, &config.protected_roots, config.ignore_hidden)?;
    let files = collected
        .files
        .into_iter()
        .filter(|f| supported_image_extension(&f.path) || supported_raw_extension(&f.path))
        .collect::<Vec<_>>();

    let duplicate_groups = group_raw_jpeg_pairs(&files);

    Ok(ScanReport {
        mode: ScanMode::RawJpegPairs,
        scanned_files: files.len(),
        candidate_size_groups: 0,
        cache_hits: 0,
        cache_misses: 0,
        duplicate_groups,
        errors: collected.errors,
        risk: MatchRisk::Medium,
    })
}

pub fn scan_empty_files(config: &ScanConfig) -> Result<ScanReport> {
    let collected = collect_files(&config.paths, &config.protected_roots, config.ignore_hidden)?;
    let files = collected.files;
    let empty_files = files
        .iter()
        .filter(|file| file.size == 0)
        .cloned()
        .collect::<Vec<_>>();
    let duplicate_groups = single_review_group(
        &empty_files,
        "empty-file",
        "empty file".to_string(),
        "empty files ready for review".to_string(),
    );

    Ok(ScanReport {
        mode: ScanMode::EmptyFiles,
        scanned_files: files.len(),
        candidate_size_groups: 0,
        cache_hits: 0,
        cache_misses: 0,
        duplicate_groups,
        errors: collected.errors,
        risk: MatchRisk::Low,
    })
}

pub fn scan_empty_folders(config: &ScanConfig) -> Result<ScanReport> {
    let directories =
        collect_empty_directories(&config.paths, &config.protected_roots, config.ignore_hidden)?;
    let scanned_files = directories.len();
    let as_files = directories
        .iter()
        .map(|entry| FileEntry {
            path: entry.path.clone(),
            size: 0,
            modified_unix: entry.modified_unix,
            identity: None,
            is_protected: entry.is_protected,
        })
        .collect::<Vec<_>>();
    let duplicate_groups = single_review_group(
        &as_files,
        "empty-folder",
        "empty folder".to_string(),
        "empty folders ready for review".to_string(),
    );

    Ok(ScanReport {
        mode: ScanMode::EmptyFolders,
        scanned_files,
        candidate_size_groups: 0,
        cache_hits: 0,
        cache_misses: 0,
        duplicate_groups,
        errors: Vec::new(),
        risk: MatchRisk::Low,
    })
}

pub fn scan_large_files(config: &ScanConfig) -> Result<ScanReport> {
    let collected = collect_files(&config.paths, &config.protected_roots, config.ignore_hidden)?;
    let files = collected.files;
    let large_files = files
        .iter()
        .filter(|file| file.size >= config.min_size)
        .cloned()
        .collect::<Vec<_>>();
    let duplicate_groups = single_review_group_with_sort(
        &large_files,
        "large-file",
        format!(">= {} bytes", config.min_size),
        format!("files at or above {} bytes", config.min_size),
        large_file_sort_key,
    );

    Ok(ScanReport {
        mode: ScanMode::LargeFiles,
        scanned_files: files.len(),
        candidate_size_groups: 0,
        cache_hits: 0,
        cache_misses: 0,
        duplicate_groups,
        errors: collected.errors,
        risk: MatchRisk::Low,
    })
}

pub fn scan_bad_extensions(config: &ScanConfig) -> Result<ScanReport> {
    let collected = collect_files(&config.paths, &config.protected_roots, config.ignore_hidden)?;
    let files = collected.files;
    let mismatched = files
        .iter()
        .filter(|file| file.size > 0)
        .filter(|file| has_bad_extension(file))
        .cloned()
        .collect::<Vec<_>>();
    let duplicate_groups = single_review_group(
        &mismatched,
        "bad-extension",
        "extension mismatch".to_string(),
        "files whose extension does not match detected content".to_string(),
    );

    Ok(ScanReport {
        mode: ScanMode::BadExtensions,
        scanned_files: files.len(),
        candidate_size_groups: 0,
        cache_hits: 0,
        cache_misses: 0,
        duplicate_groups,
        errors: collected.errors,
        risk: MatchRisk::Medium,
    })
}

pub fn scan_empty_archives(config: &ScanConfig) -> Result<ScanReport> {
    let archives = collect_zip_files(&config.paths, config.ignore_hidden);
    let scanned_files = archives.len();
    let empty_archives = archives
        .iter()
        .filter(|file| archive_is_empty(&file.path))
        .cloned()
        .collect::<Vec<_>>();
    let duplicate_groups = single_review_group(
        &empty_archives,
        "empty-archive",
        "empty archive".to_string(),
        "archives with no file members".to_string(),
    );

    Ok(ScanReport {
        mode: ScanMode::EmptyArchives,
        scanned_files,
        candidate_size_groups: 0,
        cache_hits: 0,
        cache_misses: 0,
        duplicate_groups,
        errors: Vec::new(),
        risk: MatchRisk::Low,
    })
}

pub fn scan_similar_videos(config: &ScanConfig) -> Result<ScanReport> {
    let tools = MediaToolConfig::default();
    if let Err(err) = media_tools_available(&tools) {
        return Ok(ScanReport {
            mode: ScanMode::SimilarVideos,
            scanned_files: 0,
            candidate_size_groups: 0,
            cache_hits: 0,
            cache_misses: 0,
            duplicate_groups: Vec::new(),
            errors: vec![format!("media dependency check failed: {err}")],
            risk: MatchRisk::High,
        });
    }

    let collected = collect_files(&config.paths, &config.protected_roots, config.ignore_hidden)?;
    let cache = open_cache_if_enabled(&config.cache)?;
    let files = collected
        .files
        .into_iter()
        .filter(|f| f.size >= config.min_size)
        .filter(|f| supported_video_extension(&f.path))
        .collect::<Vec<_>>();

    let scanned_files = files.len();
    let mut cache_hits = 0usize;
    let mut cache_misses = 0usize;
    let mut errors = collected.errors;
    let duplicate_groups = group_similar_video_files(
        cache.as_ref(),
        &files,
        config,
        &tools,
        &mut errors,
        &mut cache_hits,
        &mut cache_misses,
    );

    Ok(ScanReport {
        mode: ScanMode::SimilarVideos,
        scanned_files,
        candidate_size_groups: 0,
        cache_hits,
        cache_misses,
        duplicate_groups,
        errors,
        risk: MatchRisk::High,
    })
}

pub fn scan_similar_audio(config: &ScanConfig) -> Result<ScanReport> {
    let tools = MediaToolConfig::default();
    if let Err(err) = media_tools_available(&tools) {
        return Ok(ScanReport {
            mode: ScanMode::SimilarAudio,
            scanned_files: 0,
            candidate_size_groups: 0,
            cache_hits: 0,
            cache_misses: 0,
            duplicate_groups: Vec::new(),
            errors: vec![format!("media dependency check failed: {err}")],
            risk: MatchRisk::High,
        });
    }

    let collected = collect_files(&config.paths, &config.protected_roots, config.ignore_hidden)?;
    let cache = open_cache_if_enabled(&config.cache)?;
    let files = collected
        .files
        .into_iter()
        .filter(|f| f.size >= config.min_size)
        .filter(|f| supported_audio_extension(&f.path))
        .collect::<Vec<_>>();

    let scanned_files = files.len();
    let mut cache_hits = 0usize;
    let mut cache_misses = 0usize;
    let mut errors = collected.errors;
    let duplicate_groups = group_similar_audio_files(
        cache.as_ref(),
        &files,
        config,
        &tools,
        &mut errors,
        &mut cache_hits,
        &mut cache_misses,
    );

    Ok(ScanReport {
        mode: ScanMode::SimilarAudio,
        scanned_files,
        candidate_size_groups: 0,
        cache_hits,
        cache_misses,
        duplicate_groups,
        errors,
        risk: MatchRisk::High,
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

fn group_similar_image_files(
    cache: Option<&Cache>,
    files: &[FileEntry],
    config: &ScanConfig,
    errors: &mut Vec<String>,
    cache_hits: &mut usize,
    cache_misses: &mut usize,
) -> Vec<DuplicateGroup> {
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); files.len()];
    let mut pair_reasons: HashMap<(usize, usize), (u8, String)> = HashMap::new();
    let mut analyses: Vec<Option<ImageRecord>> = vec![None; files.len()];

    for index in 0..files.len() {
        if supported_image_extension(&files[index].path) {
            match image_record(cache, &files[index], config, cache_hits, cache_misses) {
                Ok(record) => analyses[index] = Some(record),
                Err(err) => errors.push(format!("{}: {err}", files[index].path.display())),
            }
        }
    }

    for left in 0..files.len() {
        for right in (left + 1)..files.len() {
            if let Some(reason) = raw_jpeg_pair_reason(&files[left], &files[right]) {
                adjacency[left].push(right);
                adjacency[right].push(left);
                pair_reasons.insert((left, right), (100, reason));
                continue;
            }

            let (Some(left_record), Some(right_record)) = (&analyses[left], &analyses[right])
            else {
                continue;
            };

            let Ok(distance) = compare_hashes_hex(&left_record.hash_hex, &right_record.hash_hex)
            else {
                continue;
            };
            if distance <= config.image_hamming_threshold {
                let score = similarity_score_from_distance(distance, config.image_hash_size);
                let mut reason = format!(
                    "similar image by perceptual hash distance {} ({}x{} aHash)",
                    distance, config.image_hash_size, config.image_hash_size
                );
                if left_record.exif_date.is_some()
                    && left_record.exif_date == right_record.exif_date
                {
                    reason.push_str(&format!(
                        " with matching EXIF date {}",
                        left_record.exif_date.clone().unwrap_or_default()
                    ));
                }
                if config.image_rotation_invariant {
                    reason.push_str("; rotation-aware comparison enabled");
                }
                adjacency[left].push(right);
                adjacency[right].push(left);
                pair_reasons.insert((left, right), (score, reason));
            }
        }
    }

    build_connected_groups(files, adjacency, pair_reasons, "similar image")
}

fn group_raw_jpeg_pairs(files: &[FileEntry]) -> Vec<DuplicateGroup> {
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); files.len()];
    let mut pair_reasons: HashMap<(usize, usize), (u8, String)> = HashMap::new();

    for left in 0..files.len() {
        for right in (left + 1)..files.len() {
            if let Some(reason) = raw_jpeg_pair_reason(&files[left], &files[right]) {
                adjacency[left].push(right);
                adjacency[right].push(left);
                pair_reasons.insert((left, right), (100, reason));
            }
        }
    }

    build_connected_groups(files, adjacency, pair_reasons, "raw+jpeg pair")
}

fn group_similar_video_files(
    cache: Option<&Cache>,
    files: &[FileEntry],
    config: &ScanConfig,
    tools: &MediaToolConfig,
    errors: &mut Vec<String>,
    cache_hits: &mut usize,
    cache_misses: &mut usize,
) -> Vec<DuplicateGroup> {
    let mut records: Vec<Option<VideoRecord>> = vec![None; files.len()];
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); files.len()];
    let mut pair_reasons: HashMap<(usize, usize), (u8, String)> = HashMap::new();

    for index in 0..files.len() {
        match video_record(
            cache,
            &files[index],
            config,
            tools,
            cache_hits,
            cache_misses,
        ) {
            Ok(record) => records[index] = Some(record),
            Err(err) => errors.push(format!("{}: {err}", files[index].path.display())),
        }
    }

    for left in 0..files.len() {
        for right in (left + 1)..files.len() {
            let (Some(left_record), Some(right_record)) = (&records[left], &records[right]) else {
                continue;
            };
            let duration_delta =
                (left_record.duration_seconds - right_record.duration_seconds).abs();
            if duration_delta > config.media_duration_tolerance_secs {
                continue;
            }

            let Ok(distance) =
                compare_hashes_hex(&left_record.fingerprint_hex, &right_record.fingerprint_hex)
            else {
                continue;
            };
            if !media_match_is_within_threshold(
                distance,
                config.media_fingerprint_distance_threshold,
            ) {
                continue;
            }
            let score = media_score_from_distance(distance);
            let reason = format!(
                "similar video by duration delta {:.2}s and sampled frame fingerprint distance {}",
                duration_delta, distance
            );
            adjacency[left].push(right);
            adjacency[right].push(left);
            pair_reasons.insert((left, right), (score, reason));
        }
    }

    build_connected_groups(files, adjacency, pair_reasons, "similar video")
}

fn group_similar_audio_files(
    cache: Option<&Cache>,
    files: &[FileEntry],
    config: &ScanConfig,
    tools: &MediaToolConfig,
    errors: &mut Vec<String>,
    cache_hits: &mut usize,
    cache_misses: &mut usize,
) -> Vec<DuplicateGroup> {
    let mut records: Vec<Option<AudioRecord>> = vec![None; files.len()];
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); files.len()];
    let mut pair_reasons: HashMap<(usize, usize), (u8, String)> = HashMap::new();

    for index in 0..files.len() {
        match audio_record(
            cache,
            &files[index],
            config,
            tools,
            cache_hits,
            cache_misses,
        ) {
            Ok(record) => records[index] = Some(record),
            Err(err) => errors.push(format!("{}: {err}", files[index].path.display())),
        }
    }

    for left in 0..files.len() {
        for right in (left + 1)..files.len() {
            let (Some(left_record), Some(right_record)) = (&records[left], &records[right]) else {
                continue;
            };
            let duration_delta =
                (left_record.duration_seconds - right_record.duration_seconds).abs();
            if duration_delta > config.media_duration_tolerance_secs {
                continue;
            }

            let Ok(distance) =
                compare_hashes_hex(&left_record.fingerprint_hex, &right_record.fingerprint_hex)
            else {
                continue;
            };
            if !media_match_is_within_threshold(
                distance,
                config.media_fingerprint_distance_threshold,
            ) {
                continue;
            }
            let score = media_score_from_distance(distance);
            let metadata_basis = shared_audio_metadata(left_record, right_record);
            let reason = if let Some(metadata_basis) = metadata_basis {
                format!(
                    "similar audio by duration delta {:.2}s, fingerprint distance {}, and metadata {}",
                    duration_delta, distance, metadata_basis
                )
            } else {
                format!(
                    "similar audio by duration delta {:.2}s and audio fingerprint distance {}",
                    duration_delta, distance
                )
            };
            adjacency[left].push(right);
            adjacency[right].push(left);
            pair_reasons.insert((left, right), (score, reason));
        }
    }

    build_connected_groups(files, adjacency, pair_reasons, "similar audio")
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

fn single_review_group(
    items: &[FileEntry],
    algorithm_label: &str,
    hash: String,
    reason: String,
) -> Vec<DuplicateGroup> {
    if items.is_empty() {
        return Vec::new();
    }

    let mut ordered = items.to_vec();
    ordered.sort_by_key(item_sort_key);
    let keep_index =
        choose_keep_index_for_indices(&ordered, &(0..ordered.len()).collect::<Vec<_>>());
    let items = ordered
        .into_iter()
        .enumerate()
        .map(|(index, item)| DuplicateItem {
            path: item.path,
            size: item.size,
            modified_unix: item.modified_unix,
            is_protected: item.is_protected,
            suggested_keep: index == keep_index,
        })
        .collect::<Vec<_>>();

    vec![DuplicateGroup {
        size: items.iter().map(|item| item.size).max().unwrap_or_default(),
        algorithm: algorithm_label.to_string(),
        hash,
        reason,
        items,
    }]
}

fn single_review_group_with_sort<K: Ord>(
    items: &[FileEntry],
    algorithm_label: &str,
    hash: String,
    reason: String,
    sort_key: impl Fn(&FileEntry) -> K,
) -> Vec<DuplicateGroup> {
    if items.is_empty() {
        return Vec::new();
    }

    let mut ordered = items.to_vec();
    ordered.sort_by_key(sort_key);
    let keep_index =
        choose_keep_index_for_indices(&ordered, &(0..ordered.len()).collect::<Vec<_>>());
    let items = ordered
        .into_iter()
        .enumerate()
        .map(|(index, item)| DuplicateItem {
            path: item.path,
            size: item.size,
            modified_unix: item.modified_unix,
            is_protected: item.is_protected,
            suggested_keep: index == keep_index,
        })
        .collect::<Vec<_>>();

    vec![DuplicateGroup {
        size: items.iter().map(|item| item.size).max().unwrap_or_default(),
        algorithm: algorithm_label.to_string(),
        hash,
        reason,
        items,
    }]
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

#[derive(Clone, Debug)]
struct ImageRecord {
    hash_hex: String,
    exif_date: Option<String>,
}

#[derive(Clone, Debug)]
struct VideoRecord {
    fingerprint_hex: String,
    duration_seconds: f64,
}

#[derive(Clone, Debug)]
struct AudioRecord {
    fingerprint_hex: String,
    duration_seconds: f64,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
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

fn raw_jpeg_pair_reason(left: &FileEntry, right: &FileEntry) -> Option<String> {
    let left_raw = supported_raw_extension(&left.path);
    let right_raw = supported_raw_extension(&right.path);
    let left_jpeg = matches!(extension_value(&left.path).as_deref(), Some("jpg" | "jpeg"));
    let right_jpeg = matches!(
        extension_value(&right.path).as_deref(),
        Some("jpg" | "jpeg")
    );

    let is_pair = (left_raw && right_jpeg) || (right_raw && left_jpeg);
    if !is_pair {
        return None;
    }

    let left_stem = normalize_name(&file_stem_like(&left.path));
    let right_stem = normalize_name(&file_stem_like(&right.path));
    if left_stem == right_stem {
        Some(format!(
            "RAW + JPEG pair by normalized basename ({} ~= {})",
            left.path.display(),
            right.path.display()
        ))
    } else {
        None
    }
}

fn similarity_score_from_distance(distance: u32, hash_size: u32) -> u8 {
    let total_bits = hash_size.saturating_mul(hash_size).max(1);
    let score = ((total_bits.saturating_sub(distance)) as f64 / total_bits as f64) * 100.0;
    score.round() as u8
}

fn media_score_from_distance(distance: u32) -> u8 {
    let max_bits = 256f64;
    let score = ((max_bits - distance.min(256) as f64) / max_bits) * 100.0;
    score.round() as u8
}

fn media_match_is_within_threshold(distance: u32, threshold: u32) -> bool {
    distance <= threshold
}

fn image_record(
    cache: Option<&Cache>,
    file: &FileEntry,
    config: &ScanConfig,
    cache_hits: &mut usize,
    cache_misses: &mut usize,
) -> Result<ImageRecord> {
    let algorithm_label =
        image_cache_label(config.image_hash_size, config.image_rotation_invariant);
    if let Some(cache) = cache {
        if let Some(found) = cache.lookup_hash(
            &CacheHashKey {
                path: &file.path,
                identity: None,
                size: file.size,
                modified_unix: file.modified_unix,
                algorithm: &algorithm_label,
                scope: HashScope::Full,
            },
            CacheLookupPolicy {
                modified_time_tolerance_secs: config.cache.modified_time_tolerance_secs,
            },
        )? {
            *cache_hits += 1;
            return Ok(ImageRecord {
                hash_hex: found.hash,
                exif_date: read_exif_date(&file.path).ok().flatten(),
            });
        }
    }

    let analysis = analyze_image(
        &file.path,
        config.image_hash_size,
        config.image_rotation_invariant,
    )?;
    if let Some(cache) = cache {
        cache.store_hash(
            &CacheHashKey {
                path: &file.path,
                identity: None,
                size: file.size,
                modified_unix: file.modified_unix,
                algorithm: &algorithm_label,
                scope: HashScope::Full,
            },
            &analysis.perceptual_hash_hex,
        )?;
    }
    *cache_misses += 1;
    Ok(ImageRecord {
        hash_hex: analysis.perceptual_hash_hex,
        exif_date: analysis.exif_date,
    })
}

fn video_record(
    cache: Option<&Cache>,
    file: &FileEntry,
    config: &ScanConfig,
    tools: &MediaToolConfig,
    cache_hits: &mut usize,
    cache_misses: &mut usize,
) -> Result<VideoRecord> {
    let algorithm_label = "video-sampled-fingerprint";
    if let Some(cache) = cache {
        if let Some(found) = cache.lookup_hash(
            &CacheHashKey {
                path: &file.path,
                identity: None,
                size: file.size,
                modified_unix: file.modified_unix,
                algorithm: algorithm_label,
                scope: HashScope::Full,
            },
            CacheLookupPolicy {
                modified_time_tolerance_secs: config.cache.modified_time_tolerance_secs,
            },
        )? {
            *cache_hits += 1;
            let metadata = probe_metadata(&file.path, tools)?;
            return Ok(VideoRecord {
                fingerprint_hex: found.hash,
                duration_seconds: metadata.duration_seconds,
            });
        }
    }

    let analysis = analyze_video(&file.path, tools)?;
    if let Some(cache) = cache {
        cache.store_hash(
            &CacheHashKey {
                path: &file.path,
                identity: None,
                size: file.size,
                modified_unix: file.modified_unix,
                algorithm: algorithm_label,
                scope: HashScope::Full,
            },
            &analysis.fingerprint_hex,
        )?;
    }
    *cache_misses += 1;
    Ok(VideoRecord {
        fingerprint_hex: analysis.fingerprint_hex,
        duration_seconds: analysis.duration_seconds,
    })
}

fn audio_record(
    cache: Option<&Cache>,
    file: &FileEntry,
    config: &ScanConfig,
    tools: &MediaToolConfig,
    cache_hits: &mut usize,
    cache_misses: &mut usize,
) -> Result<AudioRecord> {
    let algorithm_label = "audio-fingerprint";
    if let Some(cache) = cache {
        if let Some(found) = cache.lookup_hash(
            &CacheHashKey {
                path: &file.path,
                identity: None,
                size: file.size,
                modified_unix: file.modified_unix,
                algorithm: algorithm_label,
                scope: HashScope::Full,
            },
            CacheLookupPolicy {
                modified_time_tolerance_secs: config.cache.modified_time_tolerance_secs,
            },
        )? {
            *cache_hits += 1;
            let metadata = probe_metadata(&file.path, tools)?;
            return Ok(AudioRecord {
                fingerprint_hex: found.hash,
                duration_seconds: metadata.duration_seconds,
                title: metadata.title,
                artist: metadata.artist,
                album: metadata.album,
            });
        }
    }

    let analysis = analyze_audio(&file.path, tools)?;
    if let Some(cache) = cache {
        cache.store_hash(
            &CacheHashKey {
                path: &file.path,
                identity: None,
                size: file.size,
                modified_unix: file.modified_unix,
                algorithm: algorithm_label,
                scope: HashScope::Full,
            },
            &analysis.fingerprint_hex,
        )?;
    }
    *cache_misses += 1;
    Ok(AudioRecord {
        fingerprint_hex: analysis.fingerprint_hex,
        duration_seconds: analysis.duration_seconds,
        title: analysis.title,
        artist: analysis.artist,
        album: analysis.album,
    })
}

fn image_cache_label(hash_size: u32, rotation_invariant: bool) -> String {
    if rotation_invariant {
        format!("image-ahash-{hash_size}-rot")
    } else {
        format!("image-ahash-{hash_size}")
    }
}

fn shared_audio_metadata(left: &AudioRecord, right: &AudioRecord) -> Option<String> {
    let mut parts = Vec::new();
    if left.title.is_some() && left.title == right.title {
        parts.push(format!("title={}", left.title.clone().unwrap_or_default()));
    }
    if left.artist.is_some() && left.artist == right.artist {
        parts.push(format!(
            "artist={}",
            left.artist.clone().unwrap_or_default()
        ));
    }
    if left.album.is_some() && left.album == right.album {
        parts.push(format!("album={}", left.album.clone().unwrap_or_default()));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn extension_value(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

fn has_bad_extension(file: &FileEntry) -> bool {
    let Some(extension) = extension_value(&file.path) else {
        return false;
    };
    let Ok(bytes) = fs::read(&file.path) else {
        return false;
    };
    let Some(expected_extensions) = detected_extensions(&bytes) else {
        return false;
    };
    !expected_extensions
        .iter()
        .any(|expected| *expected == extension)
}

fn detected_extensions(bytes: &[u8]) -> Option<&'static [&'static str]> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(&["jpg", "jpeg"]);
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
        return Some(&["png"]);
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(&["gif"]);
    }
    if bytes.starts_with(b"BM") {
        return Some(&["bmp"]);
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some(&["webp"]);
    }
    if bytes.starts_with(b"PK\x03\x04") {
        return Some(&["zip"]);
    }
    if bytes.starts_with(b"%PDF-") {
        return Some(&["pdf"]);
    }
    if bytes.starts_with(b"ID3") {
        return Some(&["mp3"]);
    }
    if bytes.starts_with(b"fLaC") {
        return Some(&["flac"]);
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WAVE" {
        return Some(&["wav"]);
    }
    if bytes.starts_with(b"OggS") {
        return Some(&["ogg", "opus"]);
    }
    None
}

fn collect_zip_files(roots: &[PathBuf], ignore_hidden: bool) -> Vec<FileEntry> {
    let mut files = Vec::new();
    for root in roots {
        for entry in WalkDir::new(root)
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
            if !matches!(
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.to_ascii_lowercase())
                    .as_deref(),
                Some("zip")
            ) {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let modified_unix = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);
            files.push(FileEntry {
                path: entry.path().to_path_buf(),
                size: metadata.len(),
                modified_unix,
                identity: None,
                is_protected: false,
            });
        }
    }
    files
}

fn archive_is_empty(path: &Path) -> bool {
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return false;
    };
    for index in 0..archive.len() {
        let Ok(member) = archive.by_index(index) else {
            continue;
        };
        if member.is_file() {
            return false;
        }
    }
    true
}

fn open_cache_if_enabled(config: &crate::scan::CacheConfig) -> Result<Option<Cache>> {
    if !config.enabled {
        return Ok(None);
    }

    let path = config
        .path
        .as_deref()
        .unwrap_or_else(|| Path::new(".dedupeforge-cache.sqlite3"));
    Ok(Some(Cache::open(path)?))
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

fn large_file_sort_key(f: &FileEntry) -> (bool, std::cmp::Reverse<u64>, Option<i64>, String) {
    (
        !f.is_protected,
        std::cmp::Reverse(f.size),
        f.modified_unix,
        f.path.to_string_lossy().to_lowercase(),
    )
}

#[derive(Clone, Debug)]
struct EmptyDirectoryEntry {
    path: PathBuf,
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

fn collect_empty_directories(
    roots: &[PathBuf],
    protected_roots: &[PathBuf],
    ignore_hidden: bool,
) -> Result<Vec<EmptyDirectoryEntry>> {
    let protected_roots = protected_roots
        .iter()
        .map(|path| std::fs::canonicalize(path).unwrap_or_else(|_| path.clone()))
        .collect::<Vec<_>>();
    let mut empty_directories = Vec::new();

    for root in roots {
        let canonical_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.clone());
        for entry in WalkDir::new(&canonical_root)
            .follow_links(false)
            .min_depth(1)
            .into_iter()
            .filter_entry(|entry| !ignore_hidden || !is_hidden_walk_entry(entry))
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            if !entry.file_type().is_dir() {
                continue;
            }

            let path =
                std::fs::canonicalize(entry.path()).unwrap_or_else(|_| entry.path().to_path_buf());
            let is_empty = match std::fs::read_dir(&path) {
                Ok(mut children) => children.next().is_none(),
                Err(_) => false,
            };
            if !is_empty {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let modified_unix = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);
            let is_protected = protected_roots.iter().any(|root| path.starts_with(root));
            empty_directories.push(EmptyDirectoryEntry {
                path,
                modified_unix,
                is_protected,
            });
        }
    }

    empty_directories.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(empty_directories)
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
            image_hash_size: 8,
            image_hamming_threshold: 12,
            image_rotation_invariant: false,
            media_duration_tolerance_secs: 2.0,
            media_fingerprint_distance_threshold: 32,
            scan_archives: false,
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

    #[test]
    fn similar_image_scan_matches_identical_raster_images() {
        use image::{ImageBuffer, Rgb};

        let root = temp_dir("images");
        let left = root.join("left.png");
        let right = root.join("right.png");
        let mut image = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(32, 32);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let value = ((x * 5 + y * 3) % 255) as u8;
            *pixel = Rgb([value, value / 2, 255 - value]);
        }
        image.save(&left).unwrap();
        std::fs::copy(&left, &right).unwrap();

        let mut config = base_config(&root);
        config.mode = ScanMode::SimilarImages;
        config.image_hamming_threshold = 4;

        let report = scan_similar_images(&config).unwrap();

        assert_eq!(report.mode, ScanMode::SimilarImages);
        assert_eq!(report.risk, MatchRisk::High);
        assert_eq!(report.duplicate_groups.len(), 1);
        assert!(report.duplicate_groups[0]
            .reason
            .contains("perceptual hash distance"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn similar_image_scan_detects_raw_jpeg_pairs_by_basename() {
        let root = temp_dir("raw-jpeg");
        let raw = root.join("IMG_1234.CR2");
        let jpeg = root.join("IMG-1234.jpg");
        std::fs::write(&raw, b"raw").unwrap();
        std::fs::write(&jpeg, b"jpeg").unwrap();

        let mut config = base_config(&root);
        config.mode = ScanMode::SimilarImages;

        let report = scan_similar_images(&config).unwrap();

        assert_eq!(report.duplicate_groups.len(), 1);
        assert!(report.duplicate_groups[0]
            .reason
            .contains("RAW + JPEG pair"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn raw_jpeg_pair_mode_is_explicit_and_medium_risk() {
        let root = temp_dir("raw-jpeg-mode");
        let raw = root.join("IMG_5000.NEF");
        let jpeg = root.join("IMG-5000.jpeg");
        std::fs::write(&raw, b"raw").unwrap();
        std::fs::write(&jpeg, b"jpeg").unwrap();

        let mut config = base_config(&root);
        config.mode = ScanMode::RawJpegPairs;

        let report = scan_raw_jpeg_pairs(&config).unwrap();

        assert_eq!(report.mode, ScanMode::RawJpegPairs);
        assert_eq!(report.risk, MatchRisk::Medium);
        assert_eq!(report.duplicate_groups.len(), 1);
        assert!(report.duplicate_groups[0]
            .reason
            .contains("RAW + JPEG pair"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn empty_file_mode_collects_zero_byte_files() {
        let root = temp_dir("empty-files");
        std::fs::write(root.join("a.txt"), b"").unwrap();
        std::fs::write(root.join("b.txt"), b"").unwrap();
        std::fs::write(root.join("c.txt"), b"data").unwrap();

        let mut config = base_config(&root);
        config.mode = ScanMode::EmptyFiles;

        let report = scan_empty_files(&config).unwrap();

        assert_eq!(report.mode, ScanMode::EmptyFiles);
        assert_eq!(report.risk, MatchRisk::Low);
        assert_eq!(report.duplicate_groups.len(), 1);
        assert_eq!(report.duplicate_groups[0].items.len(), 2);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn empty_folder_mode_collects_only_empty_directories() {
        let root = temp_dir("empty-folders");
        let empty = root.join("empty");
        let non_empty = root.join("non-empty");
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::create_dir_all(&non_empty).unwrap();
        std::fs::write(non_empty.join("file.txt"), b"data").unwrap();

        let mut config = base_config(&root);
        config.mode = ScanMode::EmptyFolders;

        let report = scan_empty_folders(&config).unwrap();

        assert_eq!(report.mode, ScanMode::EmptyFolders);
        assert_eq!(report.risk, MatchRisk::Low);
        assert_eq!(report.duplicate_groups.len(), 1);
        assert_eq!(report.duplicate_groups[0].items.len(), 1);
        assert!(report.duplicate_groups[0].items[0].path.ends_with("empty"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shared_audio_metadata_reports_matching_fields() {
        let left = AudioRecord {
            fingerprint_hex: "aa".to_string(),
            duration_seconds: 10.0,
            title: Some("Track".to_string()),
            artist: Some("Artist".to_string()),
            album: Some("Album".to_string()),
        };
        let right = AudioRecord {
            fingerprint_hex: "bb".to_string(),
            duration_seconds: 10.1,
            title: Some("Track".to_string()),
            artist: Some("Artist".to_string()),
            album: None,
        };

        let summary = shared_audio_metadata(&left, &right).unwrap();
        assert!(summary.contains("title=Track"));
        assert!(summary.contains("artist=Artist"));
    }

    #[test]
    fn media_score_decreases_with_distance() {
        assert!(media_score_from_distance(0) > media_score_from_distance(64));
    }

    #[test]
    fn video_and_audio_similarity_require_reasonable_fingerprint_distance() {
        assert!(media_match_is_within_threshold(8, 8));
        assert!(media_match_is_within_threshold(7, 8));
        assert!(!media_match_is_within_threshold(9, 8));
    }
}
