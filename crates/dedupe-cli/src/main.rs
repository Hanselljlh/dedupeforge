use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use dedupe_actions::{
    build_dry_run_quarantine_plan, ensure_plan_valid, execute_quarantine_plan,
    load_action_plan, render_human_manifest, render_human_plan, restore_from_manifest_path,
    save_action_plan, SelectionRule,
};
use dedupe_core::{scan_exact, CacheConfig, HashAlgorithm, ScanConfig, ScanReport};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "dedupeforge")]
#[command(about = "MVP duplicate-file scanner with fast hash options and protected folders")]
struct Cli {
    paths: Vec<PathBuf>,

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

    #[arg(long)]
    save_action_plan: Option<PathBuf>,

    #[arg(long)]
    load_action_plan: Option<PathBuf>,
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
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct ScanProfile {
    preset: Option<ScanPreset>,
    paths: Option<Vec<PathBuf>>,
    protected_roots: Option<Vec<PathBuf>>,
    algorithm: Option<HashAlgorithm>,
    partial_bytes: Option<u64>,
    min_size: Option<u64>,
    ignore_hidden: Option<bool>,
    byte_verify: Option<bool>,
    cache: Option<bool>,
    cache_path: Option<PathBuf>,
    cache_mtime_tolerance_secs: Option<i64>,
    output: Option<OutputFormat>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut profile = load_profile(cli.profile.as_ref())?;
    let preset = cli.preset.or(profile.preset).unwrap_or(ScanPreset::Default);
    apply_preset_defaults(&mut profile, preset);

    if cli.clear_cache && cli.rebuild_cache {
        bail!("--clear-cache and --rebuild-cache cannot be used together");
    }
    if cli.execute_action_plan && !cli.action_plan && cli.load_action_plan.is_none() {
        bail!("--execute-action-plan requires --action-plan");
    }
    if cli.restore_manifest.is_some() && (!cli.paths.is_empty() || cli.action_plan) {
        bail!("--restore-manifest runs on its own and cannot be combined with scan/action-plan inputs");
    }
    if cli.load_action_plan.is_some() && (!cli.paths.is_empty() || cli.action_plan || cli.restore_manifest.is_some()) {
        bail!("--load-action-plan runs on its own and cannot be combined with scan, action-plan generation, or restore inputs");
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

    let paths = if cli.paths.is_empty() {
        profile.paths.unwrap_or_default()
    } else {
        cli.paths
    };

    if paths.is_empty() {
        bail!("at least one scan path is required");
    }

    let config = ScanConfig {
        paths,
        protected_roots: if cli.protected.is_empty() {
            profile.protected_roots.unwrap_or_default()
        } else {
            cli.protected
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

    let report = scan_exact(&config)?;

    if cli.action_plan {
        let plan = build_dry_run_quarantine_plan(&report, cli.selection_rule.into())?;
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

fn print_human(report: &ScanReport) {
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
