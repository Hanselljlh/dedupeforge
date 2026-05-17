use anyhow::Result;
use dedupe_actions::{
    build_dry_run_quarantine_plan, execute_quarantine_plan, restore_from_manifest_path,
    ActionManifest, ActionPlan, SelectionRule,
};
use dedupe_core::{
    scan, CacheConfig, DuplicateGroup, HashAlgorithm, ScanConfig, ScanMode, ScanReport,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuiScanProfile {
    pub name: String,
    pub mode: ScanMode,
    pub paths: Vec<PathBuf>,
    pub protected_roots: Vec<PathBuf>,
    pub ignore_patterns: Vec<String>,
    pub algorithm: HashAlgorithm,
    pub partial_bytes: u64,
    pub min_size: u64,
    pub ignore_hidden: bool,
    pub byte_verify: bool,
    pub cache_enabled: bool,
    pub cache_path: Option<PathBuf>,
    pub cache_mtime_tolerance_secs: i64,
    pub name_similarity_threshold: u8,
    pub folder_similarity_threshold: u8,
    pub image_hash_size: u32,
    pub image_hamming_threshold: u32,
    pub image_rotation_invariant: bool,
    pub media_duration_tolerance_secs: f64,
    pub media_fingerprint_distance_threshold: u32,
    pub scan_archives: bool,
}

impl Default for GuiScanProfile {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            mode: ScanMode::Exact,
            paths: Vec::new(),
            protected_roots: Vec::new(),
            ignore_patterns: Vec::new(),
            algorithm: HashAlgorithm::Blake3,
            partial_bytes: 1_048_576,
            min_size: 1,
            ignore_hidden: true,
            byte_verify: false,
            cache_enabled: false,
            cache_path: None,
            cache_mtime_tolerance_secs: 0,
            name_similarity_threshold: 85,
            folder_similarity_threshold: 85,
            image_hash_size: 8,
            image_hamming_threshold: 12,
            image_rotation_invariant: false,
            media_duration_tolerance_secs: 2.0,
            media_fingerprint_distance_threshold: 32,
            scan_archives: false,
        }
    }
}

impl GuiScanProfile {
    pub fn to_scan_config(&self) -> ScanConfig {
        ScanConfig {
            mode: self.mode,
            paths: self.paths.clone(),
            protected_roots: self.protected_roots.clone(),
            algorithm: self.algorithm,
            partial_bytes: self.partial_bytes,
            min_size: self.min_size,
            ignore_hidden: self.ignore_hidden,
            byte_verify: self.byte_verify,
            cache: CacheConfig {
                enabled: self.cache_enabled,
                path: self.cache_path.clone(),
                modified_time_tolerance_secs: self.cache_mtime_tolerance_secs,
            },
            name_similarity_threshold: self.name_similarity_threshold,
            folder_similarity_threshold: self.folder_similarity_threshold,
            image_hash_size: self.image_hash_size,
            image_hamming_threshold: self.image_hamming_threshold,
            image_rotation_invariant: self.image_rotation_invariant,
            media_duration_tolerance_secs: self.media_duration_tolerance_secs,
            media_fingerprint_distance_threshold: self.media_fingerprint_distance_threshold,
            scan_archives: self.scan_archives,
            ignore_patterns: self.ignore_patterns.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultsViewModel {
    pub mode: ScanMode,
    pub risk_label: String,
    pub summary_line: String,
    pub group_count: usize,
    pub groups: Vec<GroupViewModel>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupViewModel {
    pub index: usize,
    pub size: u64,
    pub algorithm_or_engine: String,
    pub reason: String,
    pub items: Vec<ItemViewModel>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemViewModel {
    pub path: PathBuf,
    pub size: u64,
    pub modified_unix: Option<i64>,
    pub is_protected: bool,
    pub suggested_keep: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreviewData {
    pub path: PathBuf,
    pub file_name: String,
    pub extension: Option<String>,
    pub size: u64,
    pub modified_unix: Option<i64>,
    pub is_protected: bool,
    pub suggested_keep: bool,
    pub preview_kind: String,
    pub preview_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuiSessionState {
    pub profile: GuiScanProfile,
    pub last_report: Option<ScanReport>,
    pub last_action_plan: Option<ActionPlan>,
    pub last_manifest: Option<ActionManifest>,
    pub status_message: String,
}

impl Default for GuiSessionState {
    fn default() -> Self {
        Self {
            profile: GuiScanProfile::default(),
            last_report: None,
            last_action_plan: None,
            last_manifest: None,
            status_message: "Ready".to_string(),
        }
    }
}

pub struct GuiController {
    state: GuiSessionState,
}

impl Default for GuiController {
    fn default() -> Self {
        Self::new()
    }
}

impl GuiController {
    pub fn new() -> Self {
        Self {
            state: GuiSessionState::default(),
        }
    }

    pub fn state(&self) -> &GuiSessionState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut GuiSessionState {
        &mut self.state
    }

    pub fn run_scan(&mut self) -> Result<&ScanReport> {
        let report = scan(&self.state.profile.to_scan_config())?;
        self.state.status_message = format!(
            "Scan complete: {} groups, {} errors",
            report.duplicate_groups.len(),
            report.errors.len()
        );
        self.state.last_action_plan = None;
        self.state.last_manifest = None;
        self.state.last_report = Some(report);
        Ok(self.state.last_report.as_ref().expect("report just set"))
    }

    pub fn build_action_plan(&mut self, selection_rule: SelectionRule) -> Result<&ActionPlan> {
        let report = self
            .state
            .last_report
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("run a scan before building an action plan"))?;
        if report.mode != ScanMode::Exact {
            anyhow::bail!("action plans are only supported for exact-mode scan results");
        }
        let plan = build_dry_run_quarantine_plan(report, selection_rule)?;
        self.state.status_message = format!(
            "Action plan ready: {} selected items",
            plan.summary.items_selected
        );
        self.state.last_manifest = None;
        self.state.last_action_plan = Some(plan);
        Ok(self.state.last_action_plan.as_ref().expect("plan just set"))
    }

    pub fn execute_action_plan(&mut self, quarantine_root: &Path) -> Result<&ActionManifest> {
        let plan = self
            .state
            .last_action_plan
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("build an action plan before executing it"))?;
        let manifest = execute_quarantine_plan(plan, quarantine_root)?;
        self.state.status_message = format!("Quarantine batch {} complete", manifest.batch_id);
        self.state.last_manifest = Some(manifest);
        Ok(self
            .state
            .last_manifest
            .as_ref()
            .expect("manifest just set"))
    }

    pub fn restore_manifest(&mut self, manifest_path: &Path) -> Result<&ActionManifest> {
        let manifest = restore_from_manifest_path(manifest_path)?;
        self.state.status_message = format!("Restore complete from {}", manifest_path.display());
        self.state.last_manifest = Some(manifest);
        Ok(self
            .state
            .last_manifest
            .as_ref()
            .expect("manifest just set"))
    }

    pub fn save_session(&self, path: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(&self.state)?;
        fs::write(path, text)?;
        Ok(())
    }

    pub fn load_session(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        let state: GuiSessionState = serde_json::from_str(&text)?;
        Ok(Self { state })
    }

    pub fn save_report(&self, path: &Path) -> Result<()> {
        let report = self
            .state
            .last_report
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("run or load a scan before exporting a report"))?;
        let text = serde_json::to_string_pretty(report)?;
        fs::write(path, text)?;
        Ok(())
    }

    pub fn load_report(&mut self, path: &Path) -> Result<&ScanReport> {
        let text = fs::read_to_string(path)?;
        let report: ScanReport = serde_json::from_str(&text)?;
        self.state.status_message = format!(
            "Loaded report: {} groups from {}",
            report.duplicate_groups.len(),
            path.display()
        );
        self.state.last_action_plan = None;
        self.state.last_manifest = None;
        self.state.last_report = Some(report);
        Ok(self.state.last_report.as_ref().expect("report just set"))
    }

    pub fn results_view_model(&self) -> Option<ResultsViewModel> {
        self.state.last_report.as_ref().map(report_to_view_model)
    }

    pub fn preview_for_item(&self, item: &ItemViewModel) -> PreviewData {
        build_preview_data(item)
    }
}

pub fn report_to_view_model(report: &ScanReport) -> ResultsViewModel {
    ResultsViewModel {
        mode: report.mode,
        risk_label: format!("{:?}", report.risk).to_lowercase(),
        summary_line: format!(
            "{} groups from {} scanned files",
            report.duplicate_groups.len(),
            report.scanned_files
        ),
        group_count: report.duplicate_groups.len(),
        groups: report
            .duplicate_groups
            .iter()
            .enumerate()
            .map(group_to_view_model)
            .collect(),
        errors: report.errors.clone(),
    }
}

fn group_to_view_model((index, group): (usize, &DuplicateGroup)) -> GroupViewModel {
    GroupViewModel {
        index: index + 1,
        size: group.size,
        algorithm_or_engine: group.algorithm.clone(),
        reason: group.reason.clone(),
        items: group
            .items
            .iter()
            .map(|item| ItemViewModel {
                path: item.path.clone(),
                size: item.size,
                modified_unix: item.modified_unix,
                is_protected: item.is_protected,
                suggested_keep: item.suggested_keep,
            })
            .collect(),
    }
}

fn build_preview_data(item: &ItemViewModel) -> PreviewData {
    let extension = item
        .path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());
    let file_name = item
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let (preview_kind, preview_text) = read_preview_text(&item.path, extension.as_deref());

    PreviewData {
        path: item.path.clone(),
        file_name,
        extension,
        size: item.size,
        modified_unix: item.modified_unix,
        is_protected: item.is_protected,
        suggested_keep: item.suggested_keep,
        preview_kind,
        preview_text,
    }
}

fn read_preview_text(path: &Path, extension: Option<&str>) -> (String, String) {
    let Some(kind) = preview_kind(extension) else {
        return (
            "metadata".to_string(),
            "No inline preview for this file type yet.".to_string(),
        );
    };

    match kind {
        "text" => match fs::read_to_string(path) {
            Ok(text) => ("text".to_string(), truncate_preview(&text, 1_500)),
            Err(err) => (
                "text".to_string(),
                format!("Unable to read text preview: {err}"),
            ),
        },
        "binary" => match fs::File::open(path) {
            Ok(mut file) => {
                let mut buf = [0u8; 64];
                match file.read(&mut buf) {
                    Ok(read) => (
                        "binary".to_string(),
                        buf[..read]
                            .iter()
                            .map(|byte| format!("{byte:02X}"))
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                    Err(err) => (
                        "binary".to_string(),
                        format!("Unable to read binary preview: {err}"),
                    ),
                }
            }
            Err(err) => (
                "binary".to_string(),
                format!("Unable to open file preview: {err}"),
            ),
        },
        "image" => (
            "image".to_string(),
            "Image preview placeholder. Metadata review is available; thumbnail rendering is the next enhancement.".to_string(),
        ),
        _ => (
            "metadata".to_string(),
            "No inline preview for this file type yet.".to_string(),
        ),
    }
}

fn preview_kind(extension: Option<&str>) -> Option<&'static str> {
    match extension.unwrap_or_default() {
        "txt" | "md" | "json" | "csv" | "rs" | "toml" | "yaml" | "yml" | "log" => Some("text"),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" => Some("image"),
        "bin" | "dat" | "db" | "sqlite" | "sqlite3" => Some("binary"),
        _ => None,
    }
}

fn truncate_preview(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let head = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{head}\n\n[preview truncated]")
    } else {
        head
    }
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
        let path = std::env::temp_dir().join(format!("dedupeforge-gui-{unique}-{name}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn gui_controller_runs_exact_scan_and_builds_view_model() {
        let root = temp_dir("scan");
        fs::write(root.join("a.txt"), b"same").unwrap();
        fs::write(root.join("b.txt"), b"same").unwrap();

        let mut controller = GuiController::new();
        controller.state_mut().profile.paths = vec![root.clone()];
        controller.state_mut().profile.mode = ScanMode::Exact;

        let report = controller.run_scan().unwrap();
        assert_eq!(report.mode, ScanMode::Exact);

        let view = controller.results_view_model().unwrap();
        assert_eq!(view.group_count, 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gui_controller_can_build_plan_from_exact_scan() {
        let root = temp_dir("plan");
        fs::write(root.join("a.txt"), b"same").unwrap();
        fs::write(root.join("b.txt"), b"same").unwrap();

        let mut controller = GuiController::new();
        controller.state_mut().profile.paths = vec![root.clone()];
        controller.run_scan().unwrap();

        let plan = controller
            .build_action_plan(SelectionRule::KeepSuggested)
            .unwrap();
        assert_eq!(plan.summary.items_selected, 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gui_controller_rejects_action_plan_for_non_exact_scan() {
        let root = temp_dir("plan-non-exact");
        fs::write(root.join("Vacation 2024.jpg"), b"a").unwrap();
        fs::write(root.join("vacation_2024 copy.jpg"), b"b").unwrap();

        let mut controller = GuiController::new();
        controller.state_mut().profile.paths = vec![root.clone()];
        controller.state_mut().profile.mode = ScanMode::SimilarNames;
        controller.run_scan().unwrap();

        let err = controller
            .build_action_plan(SelectionRule::KeepSuggested)
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("action plans are only supported for exact-mode scan results"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gui_session_round_trip_preserves_profile() {
        let root = temp_dir("session");
        let session_path = root.join("session.json");

        let mut controller = GuiController::new();
        controller.state_mut().profile.name = "NAS review".to_string();
        controller.state_mut().profile.paths = vec![root.clone()];
        controller.save_session(&session_path).unwrap();

        let loaded = GuiController::load_session(&session_path).unwrap();
        assert_eq!(loaded.state().profile.name, "NAS review");
        assert_eq!(loaded.state().profile.paths.len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gui_controller_can_save_and_load_report() {
        let root = temp_dir("report");
        let report_path = root.join("report.json");
        fs::write(root.join("a.txt"), b"same").unwrap();
        fs::write(root.join("b.txt"), b"same").unwrap();

        let mut controller = GuiController::new();
        controller.state_mut().profile.paths = vec![root.clone()];
        controller.run_scan().unwrap();
        controller.save_report(&report_path).unwrap();

        let mut loaded = GuiController::new();
        let report = loaded.load_report(&report_path).unwrap();
        assert_eq!(report.duplicate_groups.len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preview_data_reads_text_files() {
        let root = temp_dir("preview");
        let file_path = root.join("note.txt");
        fs::write(&file_path, "hello preview panel").unwrap();

        let preview = build_preview_data(&ItemViewModel {
            path: file_path,
            size: 19,
            modified_unix: Some(1),
            is_protected: false,
            suggested_keep: true,
        });

        assert_eq!(preview.preview_kind, "text");
        assert!(preview.preview_text.contains("hello preview panel"));

        let _ = fs::remove_dir_all(root);
    }
}
