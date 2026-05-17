use anyhow::{bail, Context, Result};
use dedupe_core::{DuplicateGroup, DuplicateItem, ScanMode, ScanReport};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ActionKind {
    QuarantineMove,
    HardlinkReplace,
    SymlinkReplace,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum SelectionRule {
    KeepSuggested,
    KeepNewest,
    KeepOldest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionPlan {
    pub version: u32,
    pub mode: String,
    #[serde(default = "default_action_kind_string")]
    pub action_kind: String,
    pub selection_rule: String,
    pub summary: ActionPlanSummary,
    pub items: Vec<ActionItem>,
    pub validation: ActionValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionPlanSummary {
    pub groups_considered: usize,
    pub items_selected: usize,
    pub protected_items_skipped: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionItem {
    pub group_id: String,
    pub action: String,
    pub path: PathBuf,
    pub size: u64,
    pub algorithm: String,
    pub hash: String,
    pub reason: String,
    pub protected: bool,
    pub suggested_keep: bool,
    pub replacement_target: Option<PathBuf>,
    pub status: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ActionValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionManifest {
    pub version: u32,
    pub batch_id: String,
    pub created_at_unix: i64,
    pub action: String,
    pub quarantine_root: PathBuf,
    pub items: Vec<ActionManifestItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionManifestItem {
    pub group_id: String,
    pub original_path: PathBuf,
    pub quarantine_path: PathBuf,
    pub size: u64,
    pub hash_algorithm: String,
    pub hash: String,
    pub replacement_target: Option<PathBuf>,
    pub status: String,
    pub error: Option<String>,
}

pub fn build_dry_run_quarantine_plan(
    report: &ScanReport,
    selection_rule: SelectionRule,
) -> Result<ActionPlan> {
    build_dry_run_plan(report, selection_rule, ActionKind::QuarantineMove)
}

pub fn build_dry_run_plan(
    report: &ScanReport,
    selection_rule: SelectionRule,
    action_kind: ActionKind,
) -> Result<ActionPlan> {
    if report.mode != ScanMode::Exact {
        bail!("action plans are only supported for exact-mode scan results");
    }

    let mut items = Vec::new();
    let mut protected_items_skipped = 0usize;

    for (group_index, group) in report.duplicate_groups.iter().enumerate() {
        let keep_index = keep_index_for_rule(group, selection_rule);
        let keep_item = group.items.get(keep_index).unwrap_or(&group.items[0]);
        for item in selected_items(group, keep_index) {
            if item.is_protected {
                protected_items_skipped += 1;
                continue;
            }

            items.push(ActionItem {
                group_id: format!("group-{group_index:04}"),
                action: action_kind_label(action_kind).to_string(),
                path: item.path.clone(),
                size: item.size,
                algorithm: group.algorithm.clone(),
                hash: group.hash.clone(),
                reason: group.reason.clone(),
                protected: item.is_protected,
                suggested_keep: item.suggested_keep,
                replacement_target: match action_kind {
                    ActionKind::QuarantineMove => None,
                    ActionKind::HardlinkReplace | ActionKind::SymlinkReplace => {
                        Some(keep_item.path.clone())
                    }
                },
                status: "planned".to_string(),
            });
        }
    }

    let validation = validate_plan(report, &items);

    Ok(ActionPlan {
        version: 1,
        mode: "dry-run".to_string(),
        action_kind: action_kind_label(action_kind).to_string(),
        selection_rule: selection_rule_label(selection_rule).to_string(),
        summary: ActionPlanSummary {
            groups_considered: report.duplicate_groups.len(),
            items_selected: items.len(),
            protected_items_skipped,
        },
        items,
        validation,
    })
}

pub fn execute_quarantine_plan(
    plan: &ActionPlan,
    quarantine_root: &Path,
) -> Result<ActionManifest> {
    ensure_plan_valid(plan)?;
    fs::create_dir_all(quarantine_root).with_context(|| {
        format!(
            "failed to create quarantine root {}",
            quarantine_root.display()
        )
    })?;

    let batch_id = batch_id_string();
    let batch_root = quarantine_root.join(&batch_id);
    let files_root = batch_root.join("files");
    fs::create_dir_all(&files_root)?;
    validate_execution_environment(plan, &batch_root)?;

    let mut manifest_items = Vec::new();
    for item in &plan.items {
        let encoded_name = encode_path_for_quarantine(&item.path);
        let destination = files_root.join(encoded_name);

        let manifest_item = match execute_action_item(item, &destination) {
            Ok(()) => ActionManifestItem {
                group_id: item.group_id.clone(),
                original_path: item.path.clone(),
                quarantine_path: destination,
                size: item.size,
                hash_algorithm: item.algorithm.clone(),
                hash: item.hash.clone(),
                replacement_target: item.replacement_target.clone(),
                status: "completed".to_string(),
                error: None,
            },
            Err(err) => ActionManifestItem {
                group_id: item.group_id.clone(),
                original_path: item.path.clone(),
                quarantine_path: destination,
                size: item.size,
                hash_algorithm: item.algorithm.clone(),
                hash: item.hash.clone(),
                replacement_target: item.replacement_target.clone(),
                status: "failed".to_string(),
                error: Some(err.to_string()),
            },
        };

        manifest_items.push(manifest_item);
    }

    let manifest = ActionManifest {
        version: 1,
        batch_id: batch_id.clone(),
        created_at_unix: unix_now(),
        action: plan.action_kind.clone(),
        quarantine_root: batch_root.clone(),
        items: manifest_items,
    };

    let manifest_path = batch_root.join("manifest.json");
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)
        .with_context(|| format!("failed to write manifest {}", manifest_path.display()))?;
    let log_path = batch_root.join("action.log");
    fs::write(&log_path, render_action_log(&manifest))
        .with_context(|| format!("failed to write action log {}", log_path.display()))?;

    Ok(manifest)
}

pub fn restore_from_manifest_path(manifest_path: &Path) -> Result<ActionManifest> {
    let text = fs::read_to_string(manifest_path)
        .with_context(|| format!("failed to read manifest {}", manifest_path.display()))?;
    let manifest: ActionManifest = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse manifest {}", manifest_path.display()))?;
    restore_from_manifest(&manifest, manifest_path)
}

pub fn restore_from_manifest(
    manifest: &ActionManifest,
    manifest_path: &Path,
) -> Result<ActionManifest> {
    let mut restored = manifest.clone();
    restored.action = "restore_from_quarantine".to_string();

    for item in &mut restored.items {
        match restore_item(item) {
            Ok(()) => {
                item.status = "restored".to_string();
                item.error = None;
            }
            Err(err) => {
                item.status = "failed".to_string();
                item.error = Some(err.to_string());
            }
        }
    }

    fs::write(manifest_path, serde_json::to_string_pretty(&restored)?)
        .with_context(|| format!("failed to update manifest {}", manifest_path.display()))?;
    if let Some(parent) = manifest_path.parent() {
        let log_path = parent.join("action.log");
        fs::write(&log_path, render_action_log(&restored))
            .with_context(|| format!("failed to update action log {}", log_path.display()))?;
    }

    Ok(restored)
}

pub fn save_action_plan(plan: &ActionPlan, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(plan)?)
        .with_context(|| format!("failed to write action plan {}", path.display()))?;
    Ok(())
}

pub fn load_action_plan(path: &Path) -> Result<ActionPlan> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read action plan {}", path.display()))?;
    let mut plan: ActionPlan = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse action plan {}", path.display()))?;
    plan.validation = validate_loaded_plan(&plan);
    Ok(plan)
}

fn selected_items(
    group: &DuplicateGroup,
    keep_index: usize,
) -> impl Iterator<Item = &DuplicateItem> {
    group
        .items
        .iter()
        .enumerate()
        .filter(move |(idx, _)| *idx != keep_index)
        .map(|(_, item)| item)
}

fn keep_index_for_rule(group: &DuplicateGroup, selection_rule: SelectionRule) -> usize {
    match selection_rule {
        SelectionRule::KeepSuggested => group
            .items
            .iter()
            .position(|item| item.suggested_keep)
            .unwrap_or(0),
        SelectionRule::KeepNewest => group
            .items
            .iter()
            .enumerate()
            .max_by_key(|(_, item)| item.modified_unix.unwrap_or(i64::MIN))
            .map(|(idx, _)| idx)
            .unwrap_or(0),
        SelectionRule::KeepOldest => group
            .items
            .iter()
            .enumerate()
            .min_by_key(|(_, item)| item.modified_unix.unwrap_or(i64::MAX))
            .map(|(idx, _)| idx)
            .unwrap_or(0),
    }
}

fn selection_rule_label(selection_rule: SelectionRule) -> &'static str {
    match selection_rule {
        SelectionRule::KeepSuggested => "keep-suggested",
        SelectionRule::KeepNewest => "keep-newest",
        SelectionRule::KeepOldest => "keep-oldest",
    }
}

fn action_kind_label(action_kind: ActionKind) -> &'static str {
    match action_kind {
        ActionKind::QuarantineMove => "quarantine_move",
        ActionKind::HardlinkReplace => "hardlink_replace",
        ActionKind::SymlinkReplace => "symlink_replace",
    }
}

fn default_action_kind_string() -> String {
    action_kind_label(ActionKind::QuarantineMove).to_string()
}

fn validate_plan(report: &ScanReport, items: &[ActionItem]) -> ActionValidation {
    let mut errors = Vec::new();

    for item in items {
        if item.protected {
            errors.push(format!(
                "{} is protected and cannot be selected",
                item.path.display()
            ));
        }
    }

    for item in items {
        match fs::metadata(&item.path) {
            Ok(metadata) => {
                if metadata.len() != item.size {
                    errors.push(format!(
                        "{} size changed from {} to {}",
                        item.path.display(),
                        item.size,
                        metadata.len()
                    ));
                }
            }
            Err(err) => errors.push(format!("{} is unavailable: {err}", item.path.display())),
        }
    }

    for (group_index, group) in report.duplicate_groups.iter().enumerate() {
        let group_id = format!("group-{group_index:04}");
        let selected = items
            .iter()
            .filter(|item| item.group_id == group_id)
            .count();
        if selected >= group.items.len() {
            errors.push(format!(
                "{group_id} selects every item in the duplicate group"
            ));
        }
        if !group.items.iter().any(|item| item.suggested_keep) {
            errors.push(format!("{group_id} does not retain a suggested keep item"));
        }
    }

    ActionValidation {
        valid: errors.is_empty(),
        errors,
    }
}

fn validate_loaded_plan(plan: &ActionPlan) -> ActionValidation {
    let mut errors = Vec::new();

    if !plan.validation.valid {
        errors.extend(plan.validation.errors.clone());
    }

    errors.extend(validate_plan_items(&plan.items));

    ActionValidation {
        valid: errors.is_empty(),
        errors,
    }
}

fn validate_plan_items(items: &[ActionItem]) -> Vec<String> {
    let mut errors = Vec::new();

    for item in items {
        if item.protected {
            errors.push(format!(
                "{} is protected and cannot be selected",
                item.path.display()
            ));
        }
    }

    for item in items {
        match fs::metadata(&item.path) {
            Ok(metadata) => {
                if metadata.len() != item.size {
                    errors.push(format!(
                        "{} size changed from {} to {}",
                        item.path.display(),
                        item.size,
                        metadata.len()
                    ));
                }
            }
            Err(err) => errors.push(format!("{} is unavailable: {err}", item.path.display())),
        }
    }

    for item in items {
        if matches!(item.action.as_str(), "hardlink_replace" | "symlink_replace") {
            match item.replacement_target.as_ref() {
                Some(target) => {
                    if !target.exists() {
                        errors.push(format!(
                            "replacement target is unavailable for {}: {}",
                            item.path.display(),
                            target.display()
                        ));
                    }
                }
                None => errors.push(format!(
                    "replacement target is missing for {}",
                    item.path.display()
                )),
            }
        }
    }

    errors
}

pub fn render_human_plan(plan: &ActionPlan) -> String {
    let mut out = String::new();
    out.push_str(&format!("Action plan mode: {}\n", plan.mode));
    out.push_str(&format!("Action kind: {}\n", plan.action_kind));
    out.push_str(&format!("Selection rule: {}\n", plan.selection_rule));
    out.push_str(&format!(
        "Groups considered: {}\nItems selected: {}\nProtected skipped: {}\nValidation: {}\n",
        plan.summary.groups_considered,
        plan.summary.items_selected,
        plan.summary.protected_items_skipped,
        if plan.validation.valid {
            "valid"
        } else {
            "invalid"
        }
    ));

    if !plan.validation.errors.is_empty() {
        out.push_str("\nValidation errors:\n");
        for err in &plan.validation.errors {
            out.push_str(&format!("  - {err}\n"));
        }
    }

    for item in &plan.items {
        out.push_str(&format!(
            "\n{} | {} | {}\n",
            item.group_id,
            item.action,
            item.path.display()
        ));
    }

    out
}

pub fn render_human_manifest(manifest: &ActionManifest) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Executed action: {}\nBatch: {}\nManifest root: {}\n",
        manifest.action,
        manifest.batch_id,
        manifest.quarantine_root.display()
    ));

    for item in &manifest.items {
        out.push_str(&format!(
            "\n{} | {} -> {}\n",
            item.status,
            item.original_path.display(),
            item.quarantine_path.display()
        ));
        if let Some(error) = &item.error {
            out.push_str(&format!("  error: {error}\n"));
        }
    }

    out
}

pub fn render_action_log(manifest: &ActionManifest) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "action={}\nbatch_id={}\ncreated_at_unix={}\nquarantine_root={}\n",
        manifest.action,
        manifest.batch_id,
        manifest.created_at_unix,
        manifest.quarantine_root.display()
    ));

    for item in &manifest.items {
        out.push_str(&format!(
            "status={} group_id={} original={} quarantine={}",
            item.status,
            item.group_id,
            item.original_path.display(),
            item.quarantine_path.display()
        ));
        if let Some(error) = &item.error {
            out.push_str(&format!(" error={error}"));
        }
        out.push('\n');
    }

    out
}

pub fn ensure_plan_valid(plan: &ActionPlan) -> Result<()> {
    let validation = validate_loaded_plan(plan);
    if !validation.valid {
        let detail = validation.errors.join("; ");
        bail!("action plan validation failed: {detail}");
    }
    Ok(())
}

fn execute_action_item(item: &ActionItem, destination: &Path) -> Result<()> {
    match item.action.as_str() {
        "quarantine_move" => move_to_quarantine(item, destination),
        "hardlink_replace" => replace_with_link(item, destination, LinkKind::Hardlink),
        "symlink_replace" => replace_with_link(item, destination, LinkKind::Symlink),
        other => bail!("unsupported action kind: {other}"),
    }
}

fn move_to_quarantine(item: &ActionItem, destination: &Path) -> Result<()> {
    if item.protected {
        bail!("protected item cannot be moved");
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&item.path, destination).with_context(|| {
        format!(
            "failed to move {} to {}",
            item.path.display(),
            destination.display()
        )
    })?;
    Ok(())
}

#[derive(Clone, Copy)]
enum LinkKind {
    Hardlink,
    Symlink,
}

fn replace_with_link(item: &ActionItem, destination: &Path, kind: LinkKind) -> Result<()> {
    let target = item
        .replacement_target
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("replacement target is required for link actions"))?;
    if item.path == *target {
        bail!("replacement target must differ from source path");
    }

    move_to_quarantine(item, destination)?;
    let link_result = create_link(target, &item.path, kind);
    if let Err(err) = link_result {
        let _ = fs::rename(destination, &item.path);
        return Err(err);
    }

    Ok(())
}

fn restore_item(item: &ActionManifestItem) -> Result<()> {
    if item.status != "completed" && item.status != "restored" {
        bail!(
            "cannot restore {} because status is {}",
            item.original_path.display(),
            item.status
        );
    }
    if !item.quarantine_path.exists() {
        bail!(
            "quarantine file is missing for {}",
            item.original_path.display()
        );
    }
    if item.original_path.exists() {
        if item.replacement_target.is_some() {
            remove_link_or_file(&item.original_path)?;
        } else {
            bail!(
                "original path already exists for {}",
                item.original_path.display()
            );
        }
    }
    if item.original_path.exists() {
        bail!(
            "original path already exists for {}",
            item.original_path.display()
        );
    }
    if let Some(parent) = item.original_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&item.quarantine_path, &item.original_path).with_context(|| {
        format!(
            "failed to restore {} to {}",
            item.quarantine_path.display(),
            item.original_path.display()
        )
    })?;
    Ok(())
}

fn validate_execution_environment(plan: &ActionPlan, root: &Path) -> Result<()> {
    match plan.action_kind.as_str() {
        "quarantine_move" => Ok(()),
        "hardlink_replace" => validate_hardlink_environment(plan),
        "symlink_replace" => validate_symlink_environment(root),
        other => bail!("unsupported action kind: {other}"),
    }
}

fn validate_hardlink_environment(plan: &ActionPlan) -> Result<()> {
    for item in &plan.items {
        let target = item.replacement_target.as_ref().ok_or_else(|| {
            anyhow::anyhow!("replacement target is required for hardlink actions")
        })?;
        if !same_filesystem(&item.path, target)? {
            bail!(
                "hardlink replacement requires the same filesystem: {} vs {}",
                item.path.display(),
                target.display()
            );
        }
    }
    Ok(())
}

fn validate_symlink_environment(root: &Path) -> Result<()> {
    let probe_source = root.join("symlink-probe-source.tmp");
    let probe_link = root.join("symlink-probe-link.tmp");
    fs::write(&probe_source, b"probe")?;
    let result = create_link(&probe_source, &probe_link, LinkKind::Symlink);
    let _ = fs::remove_file(&probe_link);
    let _ = fs::remove_file(&probe_source);
    result
}

fn remove_link_or_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)?;
        Ok(())
    } else {
        bail!("cannot remove non-file restore target {}", path.display())
    }
}

fn create_link(target: &Path, link_path: &Path, kind: LinkKind) -> Result<()> {
    if let Some(parent) = link_path.parent() {
        fs::create_dir_all(parent)?;
    }
    match kind {
        LinkKind::Hardlink => fs::hard_link(target, link_path).with_context(|| {
            format!(
                "failed to create hard link {} -> {}",
                link_path.display(),
                target.display()
            )
        })?,
        LinkKind::Symlink => create_symlink_file(target, link_path)?,
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink_file(target: &Path, link_path: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link_path).with_context(|| {
        format!(
            "failed to create symlink {} -> {}",
            link_path.display(),
            target.display()
        )
    })?;
    Ok(())
}

#[cfg(windows)]
fn create_symlink_file(target: &Path, link_path: &Path) -> Result<()> {
    std::os::windows::fs::symlink_file(target, link_path).with_context(|| {
        format!(
            "failed to create symlink {} -> {}",
            link_path.display(),
            target.display()
        )
    })?;
    Ok(())
}

fn encode_path_for_quarantine(path: &Path) -> String {
    let raw = path.to_string_lossy();
    raw.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

#[cfg(unix)]
fn same_filesystem(left: &Path, right: &Path) -> Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let left_metadata = fs::metadata(left)?;
    let right_metadata = fs::metadata(right)?;
    Ok(left_metadata.dev() == right_metadata.dev())
}

#[cfg(windows)]
fn same_filesystem(left: &Path, right: &Path) -> Result<bool> {
    fn volume_prefix(path: &Path) -> Option<String> {
        use std::path::Component;

        path.components().find_map(|component| match component {
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().to_string()),
            _ => None,
        })
    }

    let left = fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    Ok(volume_prefix(&left) == volume_prefix(&right))
}

fn batch_id_string() -> String {
    unix_now().to_string()
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use dedupe_core::{DuplicateGroup, DuplicateItem, MatchRisk, ScanMode, ScanReport};
    use std::path::PathBuf;

    fn sample_report() -> ScanReport {
        ScanReport {
            mode: ScanMode::Exact,
            scanned_files: 2,
            candidate_size_groups: 1,
            cache_hits: 0,
            cache_misses: 2,
            duplicate_groups: vec![DuplicateGroup {
                size: 4,
                algorithm: "blake3".to_string(),
                hash: "abcd".to_string(),
                reason: "same size + same full hash".to_string(),
                items: vec![
                    DuplicateItem {
                        path: PathBuf::from("keep.txt"),
                        size: 4,
                        modified_unix: Some(1),
                        is_protected: false,
                        suggested_keep: true,
                    },
                    DuplicateItem {
                        path: PathBuf::from("copy.txt"),
                        size: 4,
                        modified_unix: Some(2),
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
    fn builds_valid_dry_run_plan() {
        let root = std::env::temp_dir().join(format!("dedupe-actions-plan-{}", unix_now()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.txt"), b"same").unwrap();
        fs::write(root.join("copy.txt"), b"same").unwrap();

        let mut report = sample_report();
        report.duplicate_groups[0].items[0].path = root.join("keep.txt");
        report.duplicate_groups[0].items[1].path = root.join("copy.txt");

        let plan = build_dry_run_quarantine_plan(&report, SelectionRule::KeepSuggested).unwrap();
        assert!(plan.validation.valid);
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].action, "quarantine_move");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn skips_protected_items_in_plan() {
        let root = std::env::temp_dir().join(format!("dedupe-actions-protected-{}", unix_now()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.txt"), b"same").unwrap();
        fs::write(root.join("copy.txt"), b"same").unwrap();

        let mut report = sample_report();
        report.duplicate_groups[0].items[0].path = root.join("keep.txt");
        report.duplicate_groups[0].items[1].path = root.join("copy.txt");
        report.duplicate_groups[0].items[1].is_protected = true;

        let plan = build_dry_run_quarantine_plan(&report, SelectionRule::KeepSuggested).unwrap();
        assert!(plan.items.is_empty());
        assert_eq!(plan.summary.protected_items_skipped, 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn executes_quarantine_plan_and_writes_manifest() {
        let root = std::env::temp_dir().join(format!("dedupe-actions-exec-{}", unix_now()));
        let quarantine_root = root.join(".quarantine");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.txt"), b"same").unwrap();
        fs::write(root.join("copy.txt"), b"same").unwrap();

        let mut report = sample_report();
        report.duplicate_groups[0].items[0].path = root.join("keep.txt");
        report.duplicate_groups[0].items[1].path = root.join("copy.txt");

        let plan = build_dry_run_quarantine_plan(&report, SelectionRule::KeepSuggested).unwrap();
        let manifest = execute_quarantine_plan(&plan, &quarantine_root).unwrap();

        assert_eq!(manifest.items.len(), 1);
        assert_eq!(manifest.items[0].status, "completed");
        assert!(!report.duplicate_groups[0].items[1].path.exists());
        assert!(manifest.items[0].quarantine_path.exists());
        assert!(manifest.quarantine_root.join("manifest.json").exists());
        assert!(manifest.quarantine_root.join("action.log").exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restores_files_from_manifest() {
        let root = std::env::temp_dir().join(format!("dedupe-actions-restore-{}", unix_now()));
        let quarantine_root = root.join(".quarantine");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.txt"), b"same").unwrap();
        fs::write(root.join("copy.txt"), b"same").unwrap();

        let mut report = sample_report();
        report.duplicate_groups[0].items[0].path = root.join("keep.txt");
        report.duplicate_groups[0].items[1].path = root.join("copy.txt");

        let plan = build_dry_run_quarantine_plan(&report, SelectionRule::KeepSuggested).unwrap();
        let manifest = execute_quarantine_plan(&plan, &quarantine_root).unwrap();
        let manifest_path = manifest.quarantine_root.join("manifest.json");

        let restored = restore_from_manifest_path(&manifest_path).unwrap();

        assert_eq!(restored.items[0].status, "restored");
        assert!(root.join("copy.txt").exists());
        assert!(!restored.items[0].quarantine_path.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn keep_newest_selects_older_duplicate_for_quarantine() {
        let root = std::env::temp_dir().join(format!("dedupe-actions-newest-{}", unix_now()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("older.txt"), b"same").unwrap();
        fs::write(root.join("newer.txt"), b"same").unwrap();

        let report = ScanReport {
            mode: ScanMode::Exact,
            scanned_files: 2,
            candidate_size_groups: 1,
            cache_hits: 0,
            cache_misses: 2,
            duplicate_groups: vec![DuplicateGroup {
                size: 4,
                algorithm: "blake3".to_string(),
                hash: "abcd".to_string(),
                reason: "same size + same full hash".to_string(),
                items: vec![
                    DuplicateItem {
                        path: root.join("older.txt"),
                        size: 4,
                        modified_unix: Some(10),
                        is_protected: false,
                        suggested_keep: false,
                    },
                    DuplicateItem {
                        path: root.join("newer.txt"),
                        size: 4,
                        modified_unix: Some(20),
                        is_protected: false,
                        suggested_keep: true,
                    },
                ],
            }],
            errors: vec![],
            risk: MatchRisk::Low,
        };

        let plan = build_dry_run_quarantine_plan(&report, SelectionRule::KeepNewest).unwrap();
        assert_eq!(plan.items.len(), 1);
        assert!(plan.items[0].path.ends_with("older.txt"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_non_exact_reports_for_action_planning() {
        let mut report = sample_report();
        report.mode = ScanMode::SimilarImages;

        let err = build_dry_run_quarantine_plan(&report, SelectionRule::KeepSuggested).unwrap_err();
        assert!(err
            .to_string()
            .contains("action plans are only supported for exact-mode scan results"));
    }

    #[test]
    fn executes_hardlink_replacement_and_restores_original() {
        let root = std::env::temp_dir().join(format!("dedupe-actions-hardlink-{}", unix_now()));
        let quarantine_root = root.join(".quarantine");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.txt"), b"same").unwrap();
        fs::write(root.join("copy.txt"), b"same").unwrap();

        let mut report = sample_report();
        report.duplicate_groups[0].items[0].path = root.join("keep.txt");
        report.duplicate_groups[0].items[1].path = root.join("copy.txt");

        let plan = build_dry_run_plan(
            &report,
            SelectionRule::KeepSuggested,
            ActionKind::HardlinkReplace,
        )
        .unwrap();
        let manifest = execute_quarantine_plan(&plan, &quarantine_root).unwrap();

        assert_eq!(manifest.action, "hardlink_replace");
        assert!(root.join("copy.txt").exists());
        assert!(manifest.items[0].quarantine_path.exists());

        let restored =
            restore_from_manifest(&manifest, &manifest.quarantine_root.join("manifest.json"))
                .unwrap();
        assert_eq!(restored.items[0].status, "restored");
        assert!(root.join("copy.txt").exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn saves_action_plan_to_disk() {
        let root = std::env::temp_dir().join(format!("dedupe-actions-save-{}", unix_now()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.txt"), b"same").unwrap();
        fs::write(root.join("copy.txt"), b"same").unwrap();

        let mut report = sample_report();
        report.duplicate_groups[0].items[0].path = root.join("keep.txt");
        report.duplicate_groups[0].items[1].path = root.join("copy.txt");

        let plan = build_dry_run_quarantine_plan(&report, SelectionRule::KeepSuggested).unwrap();
        let path = root.join("plan.json");
        save_action_plan(&plan, &path).unwrap();

        assert!(path.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn loads_saved_action_plan_from_disk() {
        let root = std::env::temp_dir().join(format!("dedupe-actions-load-{}", unix_now()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.txt"), b"same").unwrap();
        fs::write(root.join("copy.txt"), b"same").unwrap();

        let mut report = sample_report();
        report.duplicate_groups[0].items[0].path = root.join("keep.txt");
        report.duplicate_groups[0].items[1].path = root.join("copy.txt");

        let plan = build_dry_run_quarantine_plan(&report, SelectionRule::KeepSuggested).unwrap();
        let path = root.join("plan.json");
        save_action_plan(&plan, &path).unwrap();

        let loaded = load_action_plan(&path).unwrap();
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.selection_rule, "keep-suggested");
        assert!(loaded.validation.valid);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn loaded_action_plan_is_revalidated_against_current_filesystem() {
        let root = std::env::temp_dir().join(format!("dedupe-actions-revalidate-{}", unix_now()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.txt"), b"same").unwrap();
        fs::write(root.join("copy.txt"), b"same").unwrap();

        let mut report = sample_report();
        report.duplicate_groups[0].items[0].path = root.join("keep.txt");
        report.duplicate_groups[0].items[1].path = root.join("copy.txt");

        let plan = build_dry_run_quarantine_plan(&report, SelectionRule::KeepSuggested).unwrap();
        let path = root.join("plan.json");
        save_action_plan(&plan, &path).unwrap();

        fs::remove_file(root.join("copy.txt")).unwrap();

        let loaded = load_action_plan(&path).unwrap();
        assert!(!loaded.validation.valid);
        assert!(loaded
            .validation
            .errors
            .iter()
            .any(|err| err.contains("is unavailable")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn execute_quarantine_plan_revalidates_before_move() {
        let root =
            std::env::temp_dir().join(format!("dedupe-actions-exec-revalidate-{}", unix_now()));
        let quarantine_root = root.join(".quarantine");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("keep.txt"), b"same").unwrap();
        fs::write(root.join("copy.txt"), b"same").unwrap();

        let mut report = sample_report();
        report.duplicate_groups[0].items[0].path = root.join("keep.txt");
        report.duplicate_groups[0].items[1].path = root.join("copy.txt");

        let plan = build_dry_run_quarantine_plan(&report, SelectionRule::KeepSuggested).unwrap();
        fs::remove_file(root.join("copy.txt")).unwrap();

        let err = execute_quarantine_plan(&plan, &quarantine_root).unwrap_err();
        assert!(err.to_string().contains("action plan validation failed"));

        let _ = fs::remove_dir_all(root);
    }
}
