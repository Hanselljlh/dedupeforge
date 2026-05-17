use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use dedupe_actions::{
    build_dry_run_plan, ensure_plan_valid, execute_quarantine_plan, load_action_plan,
    render_human_manifest, render_human_plan, restore_from_manifest_path, save_action_plan,
    ActionKind, SelectionRule,
};
use dedupe_core::{scan, CacheConfig, HashAlgorithm, MatchRisk, ScanConfig, ScanMode, ScanReport};
use dedupe_report_db::ReportDb;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "dedupeforge")]
#[command(about = "MVP duplicate-file scanner with fast hash options and protected folders")]
struct Cli {
    paths: Vec<PathBuf>,

    #[arg(long, value_enum, default_value_t = CliScanMode::Exact)]
    mode: CliScanMode,

    #[arg(long)]
    profile: Option<PathBuf>,

    #[arg(long, value_enum)]
    preset: Option<ScanPreset>,

    #[arg(long, value_enum, default_value_t = CliHash::Blake3)]
    hash: CliHash,

    #[arg(long, default_value_t = 1_048_576)]
    partial_bytes: u64,

    #[arg(long, default_value_t = 1)]
    min_size: u64,

    #[arg(long)]
    protected: Vec<PathBuf>,

    #[arg(long)]
    ignore_pattern: Vec<String>,

    #[arg(long, default_value_t = true)]
    ignore_hidden: bool,

    #[arg(long, default_value_t = false)]
    byte_verify: bool,

    #[arg(long, default_value_t = false)]
    cache: bool,

    #[arg(long)]
    no_cache: bool,

    #[arg(long)]
    cache_path: Option<PathBuf>,

    #[arg(long, default_value_t = 0)]
    cache_mtime_tolerance_secs: i64,

    #[arg(long, default_value_t = 85)]
    name_similarity_threshold: u8,

    #[arg(long, default_value_t = 85)]
    folder_similarity_threshold: u8,

    #[arg(long, default_value_t = 8)]
    image_hash_size: u32,

    #[arg(long, default_value_t = 12)]
    image_hamming_threshold: u32,

    #[arg(long, default_value_t = false)]
    image_rotation_invariant: bool,

    #[arg(long, default_value_t = 2.0)]
    media_duration_tolerance_secs: f64,

    #[arg(long, default_value_t = 32)]
    media_fingerprint_distance_threshold: u32,

    #[arg(long, default_value_t = false)]
    scan_archives: bool,

    #[arg(long, default_value_t = false)]
    clear_cache: bool,

    #[arg(long, default_value_t = false)]
    rebuild_cache: bool,

    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    output: OutputFormat,

    #[arg(long, default_value_t = false)]
    action_plan: bool,

    #[arg(long, default_value_t = false)]
    validate_action_plan: bool,

    #[arg(long, default_value_t = false)]
    execute_action_plan: bool,

    #[arg(long)]
    quarantine_root: Option<PathBuf>,

    #[arg(long)]
    restore_manifest: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = CliSelectionRule::KeepSuggested)]
    selection_rule: CliSelectionRule,

    #[arg(long, value_enum, default_value_t = CliActionKind::QuarantineMove)]
    action_type: CliActionKind,

    #[arg(long)]
    save_action_plan: Option<PathBuf>,

    #[arg(long)]
    load_action_plan: Option<PathBuf>,

    #[arg(long)]
    report_db: Option<PathBuf>,

    #[arg(long)]
    store_report_name: Option<String>,

    #[arg(long, default_value_t = false)]
    list_report_db: bool,

    #[arg(long)]
    show_report_id: Option<i64>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliHash {
    Blake3,
    Xxh3_128,
    Sha256,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliSelectionRule {
    KeepSuggested,
    KeepNewest,
    KeepOldest,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliActionKind {
    QuarantineMove,
    HardlinkReplace,
    SymlinkReplace,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum CliScanMode {
    Exact,
    SimilarNames,
    SimilarImages,
    SimilarVideos,
    SimilarAudio,
    DuplicateFolders,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
enum OutputFormat {
    Human,
    Json,
    Csv,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum ScanPreset {
    Default,
    NetworkConservative,
    NetworkTolerant,
    NasConservative,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct ScanProfile {
    mode: Option<CliScanMode>,
    preset: Option<ScanPreset>,
    paths: Option<Vec<PathBuf>>,
    protected_roots: Option<Vec<PathBuf>>,
    ignore_patterns: Option<Vec<String>>,
    algorithm: Option<HashAlgorithm>,
    partial_bytes: Option<u64>,
    min_size: Option<u64>,
    ignore_hidden: Option<bool>,
    byte_verify: Option<bool>,
    cache: Option<bool>,
    cache_path: Option<PathBuf>,
    cache_mtime_tolerance_secs: Option<i64>,
    name_similarity_threshold: Option<u8>,
    folder_similarity_threshold: Option<u8>,
    image_hash_size: Option<u32>,
    image_hamming_threshold: Option<u32>,
    image_rotation_invariant: Option<bool>,
    media_duration_tolerance_secs: Option<f64>,
    media_fingerprint_distance_threshold: Option<u32>,
    scan_archives: Option<bool>,
    output: Option<OutputFormat>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut profile = load_profile(cli.profile.as_ref())?;
    let preset = cli.preset.or(profile.preset).unwrap_or(ScanPreset::Default);
    apply_preset_defaults(&mut profile, preset);
    let effective_mode = profile.mode.unwrap_or(cli.mode);

    if cli.clear_cache && cli.rebuild_cache {
        bail!("--clear-cache and --rebuild-cache cannot be used together");
    }
    if !matches!(
        effective_mode,
        CliScanMode::Exact
            | CliScanMode::SimilarImages
            | CliScanMode::SimilarVideos
            | CliScanMode::SimilarAudio
    ) && (cli.cache
        || cli.no_cache
        || cli.cache_path.is_some()
        || cli.clear_cache
        || cli.rebuild_cache)
    {
        bail!(
            "cache options are only supported in --mode exact, --mode similar-images, --mode similar-videos, or --mode similar-audio"
        );
    }
    if cli.execute_action_plan && !cli.action_plan && cli.load_action_plan.is_none() {
        bail!("--execute-action-plan requires --action-plan");
    }
    if effective_mode != CliScanMode::Exact && cli.action_plan {
        bail!("action plans are only supported in --mode exact");
    }
    if profile.image_hash_size.unwrap_or(cli.image_hash_size) < 4 {
        bail!("--image-hash-size must be at least 4");
    }
    if profile
        .media_fingerprint_distance_threshold
        .unwrap_or(cli.media_fingerprint_distance_threshold)
        > 256
    {
        bail!("--media-fingerprint-distance-threshold must be between 0 and 256");
    }
    if cli.restore_manifest.is_some() && (!cli.paths.is_empty() || cli.action_plan) {
        bail!("--restore-manifest runs on its own and cannot be combined with scan/action-plan inputs");
    }
    if cli.load_action_plan.is_some()
        && (!cli.paths.is_empty() || cli.action_plan || cli.restore_manifest.is_some())
    {
        bail!("--load-action-plan runs on its own and cannot be combined with scan, action-plan generation, or restore inputs");
    }
    if (cli.list_report_db || cli.show_report_id.is_some()) && cli.report_db.is_none() {
        bail!("--list-report-db and --show-report-id require --report-db");
    }
    if (cli.list_report_db || cli.show_report_id.is_some())
        && (!cli.paths.is_empty()
            || cli.action_plan
            || cli.restore_manifest.is_some()
            || cli.load_action_plan.is_some())
    {
        bail!("report database browse commands run on their own and cannot be combined with scan or action inputs");
    }

    if let Some(manifest_path) = cli.restore_manifest.as_ref() {
        let restored = restore_from_manifest_path(manifest_path)?;
        match cli.output {
            OutputFormat::Human => print!("{}", render_human_manifest(&restored)),
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&restored)?),
            OutputFormat::Csv => bail!("csv output is not supported for restore"),
        }
        return Ok(());
    }

    if let Some(plan_path) = cli.load_action_plan.as_ref() {
        let plan = load_action_plan(plan_path)?;
        if cli.validate_action_plan {
            ensure_plan_valid(&plan)?;
        }
        if cli.execute_action_plan {
            let quarantine_root = cli
                .quarantine_root
                .clone()
                .unwrap_or_else(|| PathBuf::from(".quarantine"));
            let manifest = execute_quarantine_plan(&plan, &quarantine_root)?;
            match cli.output {
                OutputFormat::Human => print!("{}", render_human_manifest(&manifest)),
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&manifest)?),
                OutputFormat::Csv => bail!("csv output is not supported for action plan execution"),
            }
        } else {
            match cli.output {
                OutputFormat::Human => print!("{}", render_human_plan(&plan)),
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&plan)?),
                OutputFormat::Csv => bail!("csv output is not supported for action plans"),
            }
        }
        return Ok(());
    }

    if cli.list_report_db {
        let db = ReportDb::open(cli.report_db.as_ref().unwrap())?;
        let reports = db.list_reports()?;
        match cli.output {
            OutputFormat::Human => print_report_db_list(&reports),
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&reports)?),
            OutputFormat::Csv => print_report_db_csv(&reports)?,
        }
        return Ok(());
    }

    if let Some(report_id) = cli.show_report_id {
        let db = ReportDb::open(cli.report_db.as_ref().unwrap())?;
        let stored = db.load_report(report_id)?;
        match cli.output {
            OutputFormat::Human => {
                println!(
                    "Stored report {} | {} | {} | {} groups",
                    stored.summary.id,
                    stored.summary.name,
                    stored.summary.mode,
                    stored.summary.group_count
                );
                print_human(&stored.report);
            }
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&stored)?),
            OutputFormat::Csv => print_csv(&stored.report)?,
        }
        return Ok(());
    }

    let paths = if cli.paths.is_empty() {
        profile.paths.unwrap_or_default()
    } else {
        cli.paths
    };

    if paths.is_empty() {
        bail!("at least one scan path is required");
    }

    let config = ScanConfig {
        mode: effective_mode.into(),
        paths,
        protected_roots: if cli.protected.is_empty() {
            profile.protected_roots.unwrap_or_default()
        } else {
            cli.protected
        },
        ignore_patterns: if cli.ignore_pattern.is_empty() {
            profile.ignore_patterns.unwrap_or_default()
        } else {
            cli.ignore_pattern
        },
        algorithm: profile.algorithm.unwrap_or_else(|| cli.hash.into()),
        partial_bytes: profile.partial_bytes.unwrap_or(cli.partial_bytes),
        min_size: profile.min_size.unwrap_or(cli.min_size),
        ignore_hidden: profile.ignore_hidden.unwrap_or(cli.ignore_hidden),
        byte_verify: profile.byte_verify.unwrap_or(cli.byte_verify),
        cache: CacheConfig {
            enabled: if cli.no_cache {
                false
            } else if cli.cache {
                true
            } else {
                profile.cache.unwrap_or(false)
            },
            path: cli.cache_path.or(profile.cache_path),
            modified_time_tolerance_secs: profile
                .cache_mtime_tolerance_secs
                .unwrap_or(cli.cache_mtime_tolerance_secs),
        },
        name_similarity_threshold: profile
            .name_similarity_threshold
            .unwrap_or(cli.name_similarity_threshold),
        folder_similarity_threshold: profile
            .folder_similarity_threshold
            .unwrap_or(cli.folder_similarity_threshold),
        image_hash_size: profile.image_hash_size.unwrap_or(cli.image_hash_size),
        image_hamming_threshold: profile
            .image_hamming_threshold
            .unwrap_or(cli.image_hamming_threshold),
        image_rotation_invariant: profile
            .image_rotation_invariant
            .unwrap_or(cli.image_rotation_invariant),
        media_duration_tolerance_secs: profile
            .media_duration_tolerance_secs
            .unwrap_or(cli.media_duration_tolerance_secs),
        media_fingerprint_distance_threshold: profile
            .media_fingerprint_distance_threshold
            .unwrap_or(cli.media_fingerprint_distance_threshold),
        scan_archives: profile.scan_archives.unwrap_or(cli.scan_archives),
    };

    let cache_path = resolved_cache_path(&config.cache);

    if cli.clear_cache {
        clear_cache_file(&cache_path)?;
        println!("Cleared cache at {}", cache_path.display());
        return Ok(());
    }

    if cli.rebuild_cache {
        clear_cache_file(&cache_path)?;
    }

    let report = scan(&config)?;
    if let Some(report_db_path) = cli.report_db.as_ref() {
        let db = ReportDb::open(report_db_path)?;
        let report_name = cli
            .store_report_name
            .clone()
            .unwrap_or_else(|| format!("{}-scan", scan_mode_label(report.mode)));
        let stored_id = db.store_report(&report_name, &report)?;
        eprintln!(
            "Stored report {} in {} as id {}",
            report_name,
            report_db_path.display(),
            stored_id
        );
    }

    if cli.action_plan {
        let plan = build_dry_run_plan(
            &report,
            cli.selection_rule.into(),
            cli.action_type.into(),
        )?;
        if cli.validate_action_plan {
            ensure_plan_valid(&plan)?;
        }
        if let Some(path) = cli.save_action_plan.as_ref() {
            save_action_plan(&plan, path)?;
        }

        if cli.execute_action_plan {
            let quarantine_root = cli
                .quarantine_root
                .clone()
                .unwrap_or_else(|| PathBuf::from(".quarantine"));
            let manifest = execute_quarantine_plan(&plan, &quarantine_root)?;
            match profile.output.unwrap_or(cli.output) {
                OutputFormat::Human => {
                    print!("{}", render_human_manifest(&manifest));
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&manifest)?);
                }
                OutputFormat::Csv => {
                    bail!("csv output is not supported for action plan execution");
                }
            }
            return Ok(());
        }

        match profile.output.unwrap_or(cli.output) {
            OutputFormat::Human => {
                print!("{}", render_human_plan(&plan));
            }
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            }
            OutputFormat::Csv => {
                bail!("csv output is not supported for action plans");
            }
        }
        return Ok(());
    }

    match profile.output.unwrap_or(cli.output) {
        OutputFormat::Human => print_human(&report),
        OutputFormat::Json => print_json(&report)?,
        OutputFormat::Csv => print_csv(&report)?,
    }

    Ok(())
}

impl From<CliHash> for HashAlgorithm {
    fn from(value: CliHash) -> Self {
        match value {
            CliHash::Blake3 => HashAlgorithm::Blake3,
            CliHash::Xxh3_128 => HashAlgorithm::Xxh3_128,
            CliHash::Sha256 => HashAlgorithm::Sha256,
        }
    }
}

impl From<CliSelectionRule> for SelectionRule {
    fn from(value: CliSelectionRule) -> Self {
        match value {
            CliSelectionRule::KeepSuggested => SelectionRule::KeepSuggested,
            CliSelectionRule::KeepNewest => SelectionRule::KeepNewest,
            CliSelectionRule::KeepOldest => SelectionRule::KeepOldest,
        }
    }
}

impl From<CliActionKind> for ActionKind {
    fn from(value: CliActionKind) -> Self {
        match value {
            CliActionKind::QuarantineMove => ActionKind::QuarantineMove,
            CliActionKind::HardlinkReplace => ActionKind::HardlinkReplace,
            CliActionKind::SymlinkReplace => ActionKind::SymlinkReplace,
        }
    }
}

impl From<CliScanMode> for ScanMode {
    fn from(value: CliScanMode) -> Self {
        match value {
            CliScanMode::Exact => ScanMode::Exact,
            CliScanMode::SimilarNames => ScanMode::SimilarNames,
            CliScanMode::SimilarImages => ScanMode::SimilarImages,
            CliScanMode::SimilarVideos => ScanMode::SimilarVideos,
            CliScanMode::SimilarAudio => ScanMode::SimilarAudio,
            CliScanMode::DuplicateFolders => ScanMode::DuplicateFolders,
        }
    }
}

fn print_human(report: &ScanReport) {
    println!("Mode: {}", scan_mode_label(report.mode));
    println!("Match risk: {}", match_risk_label(report.risk));
    println!("Scanned files: {}", report.scanned_files);
    println!(
        "Candidate same-size groups: {}",
        report.candidate_size_groups
    );
    println!("Cache hits: {}", report.cache_hits);
    println!("Cache misses: {}", report.cache_misses);
    println!("Duplicate groups: {}", report.duplicate_groups.len());

    if !report.errors.is_empty() {
        println!("\nErrors:");
        for err in &report.errors {
            println!("  - {err}");
        }
    }

    for (idx, group) in report.duplicate_groups.iter().enumerate() {
        println!(
            "\nGroup {} | size={} | {} | {}",
            idx + 1,
            group.size,
            group.algorithm,
            group.reason
        );
        for item in &group.items {
            let marker = if item.suggested_keep { "KEEP" } else { "DUP " };
            let protected = if item.is_protected { " protected" } else { "" };
            println!("  [{marker}]{protected} {}", item.path.display());
        }
    }
}

fn print_json(report: &ScanReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

fn print_csv(report: &ScanReport) -> Result<()> {
    let mut writer = csv::Writer::from_writer(std::io::stdout());
    writer.write_record([
        "mode",
        "group",
        "suggested_keep",
        "protected",
        "size",
        "algorithm",
        "hash",
        "reason",
        "path",
    ])?;
    for (group_idx, group) in report.duplicate_groups.iter().enumerate() {
        for item in &group.items {
            writer.write_record([
                scan_mode_label(report.mode).to_string(),
                (group_idx + 1).to_string(),
                item.suggested_keep.to_string(),
                item.is_protected.to_string(),
                item.size.to_string(),
                group.algorithm.clone(),
                group.hash.clone(),
                group.reason.clone(),
                item.path.display().to_string(),
            ])?;
        }
    }
    writer.flush()?;
    Ok(())
}

fn print_report_db_list(reports: &[dedupe_report_db::StoredReportSummary]) {
    println!("Stored reports: {}", reports.len());
    for report in reports {
        println!(
            "  #{} | {} | mode={} | risk={} | files={} | groups={}",
            report.id,
            report.name,
            report.mode,
            report.risk,
            report.scanned_files,
            report.group_count
        );
    }
}

fn print_report_db_csv(reports: &[dedupe_report_db::StoredReportSummary]) -> Result<()> {
    let mut writer = csv::Writer::from_writer(std::io::stdout());
    writer.write_record([
        "id",
        "created_at_unix",
        "name",
        "mode",
        "risk",
        "scanned_files",
        "group_count",
    ])?;
    for report in reports {
        writer.write_record([
            report.id.to_string(),
            report.created_at_unix.to_string(),
            report.name.clone(),
            report.mode.clone(),
            report.risk.clone(),
            report.scanned_files.to_string(),
            report.group_count.to_string(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

fn scan_mode_label(mode: ScanMode) -> &'static str {
    match mode {
        ScanMode::Exact => "exact",
        ScanMode::SimilarNames => "similar-names",
        ScanMode::SimilarImages => "similar-images",
        ScanMode::SimilarVideos => "similar-videos",
        ScanMode::SimilarAudio => "similar-audio",
        ScanMode::DuplicateFolders => "duplicate-folders",
    }
}

fn match_risk_label(risk: MatchRisk) -> &'static str {
    match risk {
        MatchRisk::Low => "low",
        MatchRisk::Medium => "medium",
        MatchRisk::High => "high",
    }
}

fn load_profile(path: Option<&PathBuf>) -> Result<ScanProfile> {
    let Some(path) = path else {
        return Ok(ScanProfile::default());
    };

    let text =
        fs::read_to_string(path).map_err(|e| anyhow::anyhow!("failed to read profile: {e}"))?;
    let profile = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("failed to parse profile JSON: {e}"))?;
    Ok(profile)
}

fn apply_preset_defaults(profile: &mut ScanProfile, preset: ScanPreset) {
    match preset {
        ScanPreset::Default => {}
        ScanPreset::NetworkConservative => {
            profile.cache.get_or_insert(true);
            profile.cache_mtime_tolerance_secs.get_or_insert(0);
        }
        ScanPreset::NetworkTolerant => {
            profile.cache.get_or_insert(true);
            profile.cache_mtime_tolerance_secs.get_or_insert(2);
        }
        ScanPreset::NasConservative => {
            profile.cache.get_or_insert(true);
            profile.cache_mtime_tolerance_secs.get_or_insert(2);
            profile.byte_verify.get_or_insert(true);
        }
    }
}

fn resolved_cache_path(config: &CacheConfig) -> PathBuf {
    config
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from(".dedupeforge-cache.sqlite3"))
}

fn clear_cache_file(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}
