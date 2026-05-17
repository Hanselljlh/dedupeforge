use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use dedupe_core::{scan_exact, HashAlgorithm, ScanConfig, ScanReport};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "dedupeforge")]
#[command(about = "MVP duplicate-file scanner with fast hash options and protected folders")]
struct Cli {
    #[arg(required = true)]
    paths: Vec<PathBuf>,

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

    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    output: OutputFormat,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliHash {
    Blake3,
    Xxh3_128,
    Sha256,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
    Csv,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.paths.is_empty() {
        bail!("at least one scan path is required");
    }

    let config = ScanConfig {
        paths: cli.paths,
        protected_roots: cli.protected,
        algorithm: cli.hash.into(),
        partial_bytes: cli.partial_bytes,
        min_size: cli.min_size,
        ignore_hidden: cli.ignore_hidden,
        byte_verify: cli.byte_verify,
    };

    let report = scan_exact(&config)?;

    match cli.output {
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

fn print_human(report: &ScanReport) {
    println!("Scanned files: {}", report.scanned_files);
    println!("Candidate same-size groups: {}", report.candidate_size_groups);
    println!("Duplicate groups: {}", report.duplicate_groups.len());

    if !report.errors.is_empty() {
        println!("\nErrors:");
        for err in &report.errors {
            println!("  - {err}");
        }
    }

    for (idx, group) in report.duplicate_groups.iter().enumerate() {
        println!("\nGroup {} | size={} | {} | {}", idx + 1, group.size, group.algorithm, group.reason);
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
    writer.write_record(["group", "suggested_keep", "protected", "size", "algorithm", "hash", "reason", "path"])?;
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
