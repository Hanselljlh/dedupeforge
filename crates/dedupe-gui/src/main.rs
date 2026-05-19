use dedupe_actions::{ActionKind, SelectionRule};
use dedupe_core::{HashAlgorithm, ScanMode, ScanProgressPhase};
use dedupe_gui::{
    GroupViewModel, GuiController, LoadedReportSource, PreviewData, ResultsViewModel,
};
use eframe::egui;
use eframe::egui::{ColorImage, TextureHandle, TextureOptions};
use std::path::{Path, PathBuf};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 840.0])
            .with_min_inner_size([980.0, 700.0])
            .with_title("DedupeForge"),
        ..Default::default()
    };

    eframe::run_native(
        "DedupeForge",
        options,
        Box::new(|_cc| Ok(Box::new(DedupeForgeApp::default()))),
    )
}

struct DedupeForgeApp {
    controller: GuiController,
    paths_text: String,
    protected_text: String,
    ignore_patterns_text: String,
    cache_path_text: String,
    quarantine_root_text: String,
    session_path_text: String,
    report_path_text: String,
    report_db_path_text: String,
    report_db_name_text: String,
    report_db_filter_text: String,
    manifest_path_text: String,
    results_filter_text: String,
    selection_rule: SelectionRule,
    action_kind: ActionKind,
    selected_group_index: usize,
    selected_item_index: usize,
    selected_report_db_id: Option<i64>,
    error_message: String,
    preview_texture_path: Option<PathBuf>,
    preview_texture: Option<TextureHandle>,
}

impl Default for DedupeForgeApp {
    fn default() -> Self {
        let controller = GuiController::new();
        let mut app = Self {
            controller,
            paths_text: String::new(),
            protected_text: String::new(),
            ignore_patterns_text: String::new(),
            cache_path_text: String::new(),
            quarantine_root_text: ".quarantine".to_string(),
            session_path_text: "dedupeforge-gui-session.json".to_string(),
            report_path_text: "dedupeforge-report.json".to_string(),
            report_db_path_text: ".dedupeforge-reports.sqlite3".to_string(),
            report_db_name_text: "gui-report".to_string(),
            report_db_filter_text: String::new(),
            manifest_path_text: String::new(),
            results_filter_text: String::new(),
            selection_rule: SelectionRule::KeepSuggested,
            action_kind: ActionKind::QuarantineMove,
            selected_group_index: 0,
            selected_item_index: 0,
            selected_report_db_id: None,
            error_message: String::new(),
            preview_texture_path: None,
            preview_texture: None,
        };
        app.load_profile_buffers_from_state();
        app
    }
}

impl DedupeForgeApp {
    fn reset_result_selection(&mut self) {
        self.selected_group_index = 0;
        self.selected_item_index = 0;
        self.preview_texture_path = None;
        self.preview_texture = None;
    }

    fn sync_report_db_inputs_from_state(&mut self) {
        self.report_db_path_text = self
            .controller
            .state()
            .report_db_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| ".dedupeforge-reports.sqlite3".to_string());
        self.selected_report_db_id = self
            .controller
            .state()
            .stored_reports
            .first()
            .map(|report| report.id);
    }

    fn default_report_db_name(&self) -> String {
        let profile_name = self.controller.state().profile.name.trim();
        if profile_name.is_empty() {
            "gui-report".to_string()
        } else {
            profile_name
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() {
                        ch.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .trim_matches('-')
                .to_string()
        }
    }

    fn load_profile_buffers_from_state(&mut self) {
        let profile = &self.controller.state().profile;
        self.paths_text = join_paths(&profile.paths);
        self.protected_text = join_paths(&profile.protected_roots);
        self.ignore_patterns_text = profile.ignore_patterns.join("\n");
        self.cache_path_text = profile
            .cache_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        self.sync_report_db_inputs_from_state();
        if self.report_db_name_text.trim().is_empty() || self.report_db_name_text == "gui-report" {
            self.report_db_name_text = self.default_report_db_name();
        }
    }

    fn apply_profile_buffers(&mut self) {
        let state = self.controller.state_mut();
        let profile = &mut state.profile;
        profile.paths = parse_paths(&self.paths_text);
        profile.protected_roots = parse_paths(&self.protected_text);
        profile.ignore_patterns = parse_lines(&self.ignore_patterns_text);
        profile.cache_path = parse_optional_path(&self.cache_path_text);
    }

    fn run_scan(&mut self) {
        self.apply_profile_buffers();
        match self.controller.run_scan() {
            Ok(_) => {
                self.error_message.clear();
                self.reset_result_selection();
            }
            Err(err) => self.error_message = err.to_string(),
        }
    }

    fn cancel_scan(&mut self) {
        if self.controller.cancel_scan() {
            self.error_message.clear();
        }
    }

    fn build_action_plan(&mut self) {
        match self
            .controller
            .build_action_plan(self.selection_rule, self.action_kind)
        {
            Ok(_) => self.error_message.clear(),
            Err(err) => self.error_message = err.to_string(),
        }
    }

    fn execute_action_plan(&mut self) {
        let quarantine_root = PathBuf::from(self.quarantine_root_text.trim());
        match self.controller.execute_action_plan(&quarantine_root) {
            Ok(manifest) => {
                self.error_message.clear();
                self.manifest_path_text = manifest
                    .quarantine_root
                    .join("manifest.json")
                    .display()
                    .to_string();
            }
            Err(err) => self.error_message = err.to_string(),
        }
    }

    fn restore_manifest(&mut self) {
        match self
            .controller
            .restore_manifest(Path::new(self.manifest_path_text.trim()))
        {
            Ok(_) => self.error_message.clear(),
            Err(err) => self.error_message = err.to_string(),
        }
    }

    fn save_session(&mut self) {
        match self
            .controller
            .save_session(Path::new(self.session_path_text.trim()))
        {
            Ok(()) => self.error_message.clear(),
            Err(err) => self.error_message = err.to_string(),
        }
    }

    fn load_session(&mut self) {
        match GuiController::load_session(Path::new(self.session_path_text.trim())) {
            Ok(controller) => {
                self.controller = controller;
                self.load_profile_buffers_from_state();
                self.error_message.clear();
                self.reset_result_selection();
            }
            Err(err) => self.error_message = err.to_string(),
        }
    }

    fn save_report(&mut self) {
        match self
            .controller
            .save_report(Path::new(self.report_path_text.trim()))
        {
            Ok(()) => self.error_message.clear(),
            Err(err) => self.error_message = err.to_string(),
        }
    }

    fn load_report(&mut self) {
        match self
            .controller
            .load_report(Path::new(self.report_path_text.trim()))
        {
            Ok(_) => {
                self.error_message.clear();
                self.reset_result_selection();
            }
            Err(err) => self.error_message = err.to_string(),
        }
    }

    fn refresh_report_db(&mut self) {
        match self
            .controller
            .refresh_report_db(Path::new(self.report_db_path_text.trim()))
        {
            Ok(reports) => {
                self.error_message.clear();
                self.selected_report_db_id = reports.first().map(|report| report.id);
            }
            Err(err) => self.error_message = err.to_string(),
        }
    }

    fn store_report_in_db(&mut self) {
        match self.controller.store_report_in_db(
            Path::new(self.report_db_path_text.trim()),
            self.report_db_name_text.as_str(),
        ) {
            Ok(id) => {
                self.error_message.clear();
                self.selected_report_db_id = Some(id);
                self.sync_report_db_inputs_from_state();
            }
            Err(err) => self.error_message = err.to_string(),
        }
    }

    fn open_report_from_db(&mut self) {
        let Some(id) = self.selected_report_db_id else {
            self.error_message = "Select a stored report first".to_string();
            return;
        };

        match self
            .controller
            .load_report_from_db(Path::new(self.report_db_path_text.trim()), id)
        {
            Ok(_) => {
                self.error_message.clear();
                self.reset_result_selection();
                self.selected_report_db_id = Some(id);
            }
            Err(err) => self.error_message = err.to_string(),
        }
    }
}

impl eframe::App for DedupeForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.controller.poll_scan_progress() {
            Ok(true) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
            Ok(false) => {}
            Err(err) => self.error_message = err.to_string(),
        }

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("DedupeForge");
                ui.separator();
                ui.label(self.controller.state().status_message.as_str());
                if !self.error_message.is_empty() {
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::from_rgb(190, 60, 45),
                        self.error_message.as_str(),
                    );
                }
            });
        });

        egui::SidePanel::left("left_controls")
            .resizable(true)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.heading("Scan Setup");
                ui.add_space(8.0);

                let profile = &mut self.controller.state_mut().profile;

                ui.label("Profile Name");
                ui.text_edit_singleline(&mut profile.name);

                ui.add_space(8.0);
                ui.label("Scan Mode");
                egui::ComboBox::from_id_salt("scan_mode")
                    .selected_text(scan_mode_label(profile.mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut profile.mode, ScanMode::Exact, "Exact duplicates");
                        ui.selectable_value(
                            &mut profile.mode,
                            ScanMode::SimilarNames,
                            "Similar filenames",
                        );
                        ui.selectable_value(
                            &mut profile.mode,
                            ScanMode::SimilarImages,
                            "Similar images",
                        );
                        ui.selectable_value(
                            &mut profile.mode,
                            ScanMode::SimilarVideos,
                            "Similar videos",
                        );
                        ui.selectable_value(
                            &mut profile.mode,
                            ScanMode::SimilarAudio,
                            "Similar audio",
                        );
                        ui.selectable_value(
                            &mut profile.mode,
                            ScanMode::DuplicateFolders,
                            "Duplicate folders",
                        );
                    });

                ui.add_space(8.0);
                ui.label("Source Paths");
                ui.add(
                    egui::TextEdit::multiline(&mut self.paths_text)
                        .desired_rows(4)
                        .hint_text("One folder path per line"),
                );

                ui.label("Protected Paths");
                ui.add(
                    egui::TextEdit::multiline(&mut self.protected_text)
                        .desired_rows(3)
                        .hint_text("One protected path per line"),
                );

                ui.label("Ignored Patterns");
                ui.add(
                    egui::TextEdit::multiline(&mut self.ignore_patterns_text)
                        .desired_rows(3)
                        .hint_text("Examples: *.tmp\nThumbs.db"),
                );

                ui.checkbox(&mut profile.ignore_hidden, "Ignore hidden files");
                ui.checkbox(&mut profile.byte_verify, "Byte verify exact matches");

                ui.add_space(8.0);
                ui.collapsing("Exact Scan Settings", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Hash");
                        egui::ComboBox::from_id_salt("hash_algorithm")
                            .selected_text(hash_algorithm_label(profile.algorithm))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut profile.algorithm,
                                    HashAlgorithm::Blake3,
                                    "BLAKE3",
                                );
                                ui.selectable_value(
                                    &mut profile.algorithm,
                                    HashAlgorithm::Xxh3_128,
                                    "XXH3 128",
                                );
                                ui.selectable_value(
                                    &mut profile.algorithm,
                                    HashAlgorithm::Sha256,
                                    "SHA-256",
                                );
                            });
                    });
                    ui.add(
                        egui::Slider::new(&mut profile.partial_bytes, 1_024..=8_388_608)
                            .text("Partial hash bytes"),
                    );
                    ui.add(
                        egui::Slider::new(&mut profile.min_size, 0..=16_777_216)
                            .text("Minimum file size"),
                    );
                    ui.checkbox(&mut profile.cache_enabled, "Enable cache");
                    ui.checkbox(&mut profile.scan_archives, "Scan zip archives");
                    ui.label("Cache Path");
                    ui.text_edit_singleline(&mut self.cache_path_text);
                    ui.add(
                        egui::Slider::new(&mut profile.cache_mtime_tolerance_secs, 0..=10)
                            .text("Cache mtime tolerance (secs)"),
                    );
                });

                ui.collapsing("Similarity Settings", |ui| {
                    ui.add(
                        egui::Slider::new(&mut profile.name_similarity_threshold, 50..=100)
                            .text("Name similarity threshold"),
                    );
                    ui.add(
                        egui::Slider::new(&mut profile.folder_similarity_threshold, 50..=100)
                            .text("Folder similarity threshold"),
                    );
                    ui.add(
                        egui::Slider::new(&mut profile.image_hash_size, 4..=16)
                            .text("Image hash size"),
                    );
                    ui.add(
                        egui::Slider::new(&mut profile.image_hamming_threshold, 0..=64)
                            .text("Image Hamming threshold"),
                    );
                    ui.checkbox(
                        &mut profile.image_rotation_invariant,
                        "Rotation/flip-aware image comparison",
                    );
                    ui.add(
                        egui::Slider::new(&mut profile.media_duration_tolerance_secs, 0.0..=10.0)
                            .text("Media duration tolerance (secs)"),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut profile.media_fingerprint_distance_threshold,
                            0..=256,
                        )
                        .text("Media fingerprint distance"),
                    );
                });

                ui.add_space(8.0);
                let scan_in_progress = self.controller.is_scan_in_progress();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!scan_in_progress, egui::Button::new(if scan_in_progress {
                            "Scanning..."
                        } else {
                            "Run Scan"
                        }))
                        .clicked()
                    {
                        self.run_scan();
                    }
                    if ui
                        .add_enabled(scan_in_progress, egui::Button::new("Stop Scan"))
                        .clicked()
                    {
                        self.cancel_scan();
                    }
                });

                if let Some(progress) = self.controller.state().scan_progress.as_ref() {
                    ui.add_space(6.0);
                    let fraction = progress_fraction(progress.phase, progress.current, progress.total);
                    ui.add(
                        egui::ProgressBar::new(fraction)
                            .show_percentage()
                            .text(progress.message.as_str()),
                    );
                    ui.label(format!(
                        "Phase: {} | {}/{}",
                        progress_phase_label(progress.phase),
                        progress.current,
                        progress.total
                    ));
                }

                ui.separator();
                ui.heading("Actions");
                let exact_mode_actions = self.controller.state().profile.mode == ScanMode::Exact;
                ui.horizontal(|ui| {
                    ui.label("Keep Rule");
                    egui::ComboBox::from_id_salt("selection_rule")
                        .selected_text(selection_rule_label(self.selection_rule))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.selection_rule,
                                SelectionRule::KeepSuggested,
                                "Keep suggested",
                            );
                            ui.selectable_value(
                                &mut self.selection_rule,
                                SelectionRule::KeepNewest,
                                "Keep newest",
                            );
                            ui.selectable_value(
                                &mut self.selection_rule,
                                SelectionRule::KeepOldest,
                                "Keep oldest",
                            );
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Action Type");
                    egui::ComboBox::from_id_salt("action_kind")
                        .selected_text(action_kind_label(self.action_kind))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.action_kind,
                                ActionKind::QuarantineMove,
                                "Quarantine move",
                            );
                            ui.selectable_value(
                                &mut self.action_kind,
                                ActionKind::HardlinkReplace,
                                "Hardlink replace",
                            );
                            ui.selectable_value(
                                &mut self.action_kind,
                                ActionKind::SymlinkReplace,
                                "Symlink replace",
                            );
                        });
                });
                if !exact_mode_actions {
                    ui.colored_label(
                        egui::Color32::from_rgb(196, 110, 32),
                        "Action plans and file replacement actions are only enabled for exact duplicate scans.",
                    );
                }
                if ui
                    .add_enabled(exact_mode_actions, egui::Button::new("Build Action Plan"))
                    .clicked()
                {
                    self.build_action_plan();
                }
                ui.label("Quarantine Root");
                ui.text_edit_singleline(&mut self.quarantine_root_text);
                if ui
                    .add_enabled(
                        exact_mode_actions,
                        egui::Button::new(execute_action_button_label(self.action_kind)),
                    )
                    .clicked()
                {
                    self.execute_action_plan();
                }
                ui.label("Manifest Path");
                ui.text_edit_singleline(&mut self.manifest_path_text);
                if ui.button("Restore Manifest").clicked() {
                    self.restore_manifest();
                }

                ui.separator();
                ui.heading("Session");
                ui.label("Session File");
                ui.text_edit_singleline(&mut self.session_path_text);
                ui.horizontal(|ui| {
                    if ui.button("Save Session").clicked() {
                        self.save_session();
                    }
                    if ui.button("Load Session").clicked() {
                        self.load_session();
                    }
                });

                ui.add_space(8.0);
                ui.heading("Reports");
                ui.label("Report File");
                ui.text_edit_singleline(&mut self.report_path_text);
                let has_report = self.controller.state().last_report.is_some();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(has_report, egui::Button::new("Export Report"))
                        .clicked()
                    {
                        self.save_report();
                    }
                    if ui.button("Open Report").clicked() {
                        self.load_report();
                    }
                });
                ui.add_space(8.0);
                ui.label("Report Database");
                ui.text_edit_singleline(&mut self.report_db_path_text);
                ui.horizontal(|ui| {
                    ui.label("Stored Name");
                    ui.text_edit_singleline(&mut self.report_db_name_text);
                });
                ui.horizontal(|ui| {
                    if ui.button("Refresh DB").clicked() {
                        self.refresh_report_db();
                    }
                    if ui
                        .add_enabled(has_report, egui::Button::new("Store In DB"))
                        .clicked()
                    {
                        self.store_report_in_db();
                    }
                    if ui
                        .add_enabled(
                            self.selected_report_db_id.is_some(),
                            egui::Button::new("Open From DB"),
                        )
                        .clicked()
                    {
                        self.open_report_from_db();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Saved Report Filter");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.report_db_filter_text)
                            .hint_text("Filter by id, name, mode, or risk"),
                    );
                    if ui.button("Clear").clicked() {
                        self.report_db_filter_text.clear();
                    }
                });
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        let visible_reports = filtered_report_entries(
                            &self.controller.state().stored_reports,
                            self.report_db_filter_text.as_str(),
                        );
                        if visible_reports.is_empty() {
                            ui.label("No stored reports loaded yet.");
                        } else {
                            for report in visible_reports {
                                let selected = self.selected_report_db_id == Some(report.id);
                                if ui
                                    .selectable_label(
                                        selected,
                                        format!(
                                            "#{} | {} | {} | {} groups | files={} | {}",
                                            report.id,
                                            report.name,
                                            report.mode,
                                            report.group_count,
                                            report.scanned_files,
                                            report.risk
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.selected_report_db_id = Some(report.id);
                                }
                            }
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let view = self.controller.results_view_model();

            ui.heading("Results");
            if let Some(view) = view {
                if let Some(source) = self.controller.state().last_report_source.as_ref() {
                    ui.label(report_source_label(source));
                    ui.add_space(6.0);
                }
                let filtered_group_indices =
                    filtered_group_indices(&view, self.results_filter_text.as_str());
                ui.horizontal(|ui| {
                    ui.label("Filter");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.results_filter_text)
                            .hint_text("Match group reason or file path"),
                    );
                    if ui.button("Clear").clicked() {
                        self.results_filter_text.clear();
                    }
                    if !filtered_group_indices.is_empty()
                        && filtered_group_indices.len() < view.groups.len()
                        && ui.button("Remove Filtered Groups").clicked()
                    {
                        match self
                            .controller
                            .remove_groups_from_review(&filtered_group_indices)
                        {
                            Ok(_) => {
                                self.error_message.clear();
                                self.results_filter_text.clear();
                                self.selected_group_index = 0;
                                self.selected_item_index = 0;
                            }
                            Err(err) => self.error_message = err.to_string(),
                        }
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("Mode: {}", scan_mode_label(view.mode)));
                    ui.separator();
                    ui.label(format!("Risk: {}", view.risk_label));
                    ui.separator();
                    ui.label(view.summary_line.as_str());
                    ui.separator();
                    ui.label(format!("Files: {}", view.scanned_files));
                    ui.separator();
                    ui.label(format!(
                        "Cache: {} hits / {} misses",
                        view.cache_hits, view.cache_misses
                    ));
                    if !view.errors.is_empty() {
                        ui.separator();
                        ui.label(format!("Errors: {}", view.errors.len()));
                    }
                    if filtered_group_indices.len() != view.groups.len() {
                        ui.separator();
                        ui.label(format!(
                            "Showing {} of {} groups",
                            filtered_group_indices.len(),
                            view.groups.len()
                        ));
                        ui.separator();
                        ui.label("Tip: remove filtered groups to prune the review set");
                    }
                    if matches!(
                        view.mode,
                        ScanMode::SimilarImages | ScanMode::SimilarVideos | ScanMode::SimilarAudio
                    ) {
                        ui.separator();
                        ui.colored_label(
                            egui::Color32::from_rgb(196, 110, 32),
                            "Warning: media similarity matches can produce false positives.",
                        );
                    }
                });
                ui.add_space(10.0);
                if !filtered_group_indices.contains(&self.selected_group_index) {
                    if let Some(first_index) = filtered_group_indices.first() {
                        self.selected_group_index = *first_index;
                        self.selected_item_index = 0;
                    }
                }

                ui.columns(2, |columns| {
                    columns[0].heading("Groups");
                    columns[0].separator();
                    egui::ScrollArea::vertical().show(&mut columns[0], |ui| {
                        for &group_index in &filtered_group_indices {
                            let group = &view.groups[group_index];
                            let selected = self.selected_group_index == group_index;
                            let label = format!(
                                "{}. {} items | {}",
                                group.index,
                                group.items.len(),
                                truncate_text(&group.reason, 58)
                            );
                            if ui.selectable_label(selected, label).clicked() {
                                self.selected_group_index = group_index;
                                self.selected_item_index = 0;
                            }
                        }
                    });

                    columns[1].heading("Group Details");
                    columns[1].separator();
                    if let Some(group) = view.groups.get(self.selected_group_index) {
                        if render_group_details(
                            &mut columns[1],
                            ctx,
                            group,
                            &mut self.selected_item_index,
                            &self.controller,
                            &mut self.preview_texture_path,
                            &mut self.preview_texture,
                        ) {
                            if let Err(err) = self.controller.set_keep_override(
                                self.selected_group_index,
                                self.selected_item_index,
                            ) {
                                self.error_message = err.to_string();
                            } else {
                                self.error_message.clear();
                            }
                        }
                    } else {
                        columns[1].label("Run a scan to populate results.");
                    }
                });

                if !view.errors.is_empty() {
                    ui.add_space(12.0);
                    ui.heading("Errors");
                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for error in &view.errors {
                                ui.colored_label(
                                    egui::Color32::from_rgb(190, 60, 45),
                                    error.as_str(),
                                );
                            }
                        });
                }
            } else {
                ui.label("Run a scan from the left panel to populate results.");
            }

            ui.add_space(12.0);
            if let Some(plan) = &self.controller.state().last_action_plan {
                ui.separator();
                ui.heading("Action Plan Summary");
                ui.label(format!(
                    "{} items selected across {} groups",
                    plan.summary.items_selected, plan.summary.groups_considered
                ));
                ui.label(format!(
                    "Action: {} | Keep rule: {}",
                    plan.action_kind, plan.selection_rule
                ));
                if !plan.validation.valid {
                    ui.colored_label(
                        egui::Color32::from_rgb(190, 60, 45),
                        format!("Validation failed: {}", plan.validation.errors.join("; ")),
                    );
                }
            }

            if let Some(manifest) = &self.controller.state().last_manifest {
                ui.separator();
                ui.heading("Last Manifest");
                ui.label(format!("Batch: {}", manifest.batch_id));
                ui.label(format!("Items: {}", manifest.items.len()));
            }
        });
    }
}

fn render_group_details(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    group: &GroupViewModel,
    selected_item_index: &mut usize,
    controller: &GuiController,
    preview_texture_path: &mut Option<PathBuf>,
    preview_texture: &mut Option<TextureHandle>,
) -> bool {
    ui.label(format!("Reason: {}", group.reason));
    ui.label(format!("Engine: {}", group.algorithm_or_engine));
    ui.label(format!("Representative size: {} bytes", group.size));
    ui.add_space(8.0);

    egui::ScrollArea::vertical()
        .max_height(220.0)
        .show(ui, |ui| {
            egui::Grid::new("group_items_grid")
                .num_columns(4)
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.strong("Keep");
                    ui.strong("Protected");
                    ui.strong("Size");
                    ui.strong("Path");
                    ui.end_row();

                    for (index, item) in group.items.iter().enumerate() {
                        let selected = *selected_item_index == index;
                        if ui
                            .selectable_label(
                                selected,
                                if item.suggested_keep { "Yes" } else { "" },
                            )
                            .clicked()
                        {
                            *selected_item_index = index;
                        }
                        ui.label(if item.is_protected { "Yes" } else { "" });
                        ui.label(item.size.to_string());
                        if ui
                            .selectable_label(selected, item.path.display().to_string())
                            .clicked()
                        {
                            *selected_item_index = index;
                        }
                        ui.end_row();
                    }
                });
        });

    let mut keeper_override_requested = false;
    if group.items.get(*selected_item_index).is_some() {
        if ui.button("Make Selected Keeper").clicked() {
            keeper_override_requested = true;
        }
    }

    ui.separator();
    ui.heading("Preview");
    if let Some(item) = group.items.get(*selected_item_index) {
        let preview = controller.preview_for_item(item);
        render_preview_panel(ui, ctx, &preview, preview_texture_path, preview_texture);
    } else {
        ui.label("Select a file in this group to preview it.");
    }
    keeper_override_requested
}

fn render_preview_panel(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    preview: &PreviewData,
    preview_texture_path: &mut Option<PathBuf>,
    preview_texture: &mut Option<TextureHandle>,
) {
    ui.label(format!("File: {}", preview.file_name));
    ui.label(format!("Path: {}", preview.path.display()));
    ui.label(format!("Size: {} bytes", preview.size));
    ui.label(format!(
        "Modified: {}",
        preview
            .modified_unix
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    ui.label(format!(
        "Flags: {}{}",
        if preview.suggested_keep {
            "suggested keep"
        } else {
            "candidate"
        },
        if preview.is_protected {
            ", protected"
        } else {
            ""
        }
    ));
    let extension_label = preview
        .extension
        .as_ref()
        .map(|ext| format!(" (.{ext})"))
        .unwrap_or_default();
    ui.label(format!(
        "Preview kind: {}{}",
        preview.preview_kind, extension_label
    ));
    ui.add_space(6.0);

    if preview.preview_kind == "image" {
        if let (Some(width), Some(height), Some(rgba)) = (
            preview.image_width,
            preview.image_height,
            preview.image_rgba.as_ref(),
        ) {
            let needs_reload = preview_texture_path.as_ref() != Some(&preview.path);
            if needs_reload {
                let image = ColorImage::from_rgba_unmultiplied([width, height], rgba);
                *preview_texture = Some(ctx.load_texture(
                    format!("preview:{}", preview.path.display()),
                    image,
                    TextureOptions::LINEAR,
                ));
                *preview_texture_path = Some(preview.path.clone());
            }

            if let Some(texture) = preview_texture.as_ref() {
                let available_width = ui.available_width().max(120.0);
                let scale = (available_width / width as f32).min(1.0);
                let image_size = egui::vec2(width as f32 * scale, height as f32 * scale);
                ui.image((texture.id(), image_size));
                ui.add_space(6.0);
            }
        } else {
            *preview_texture = None;
            *preview_texture_path = None;
        }
    } else {
        *preview_texture = None;
        *preview_texture_path = None;
    }

    let mut preview_buffer = preview.preview_text.clone();
    egui::ScrollArea::vertical()
        .max_height(220.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut preview_buffer)
                    .desired_rows(10)
                    .font(egui::TextStyle::Monospace)
                    .interactive(false),
            );
        });
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

fn hash_algorithm_label(algorithm: HashAlgorithm) -> &'static str {
    match algorithm {
        HashAlgorithm::Blake3 => "BLAKE3",
        HashAlgorithm::Xxh3_128 => "XXH3 128",
        HashAlgorithm::Sha256 => "SHA-256",
    }
}

fn selection_rule_label(rule: SelectionRule) -> &'static str {
    match rule {
        SelectionRule::KeepSuggested => "Keep suggested",
        SelectionRule::KeepNewest => "Keep newest",
        SelectionRule::KeepOldest => "Keep oldest",
    }
}

fn action_kind_label(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::QuarantineMove => "Quarantine move",
        ActionKind::HardlinkReplace => "Hardlink replace",
        ActionKind::SymlinkReplace => "Symlink replace",
    }
}

fn progress_phase_label(phase: ScanProgressPhase) -> &'static str {
    match phase {
        ScanProgressPhase::CollectingFiles => "Collecting files",
        ScanProgressPhase::GroupingBySize => "Grouping by size",
        ScanProgressPhase::PartialHashing => "Partial hashing",
        ScanProgressPhase::FullHashing => "Full hashing",
        ScanProgressPhase::ByteVerifying => "Byte verifying",
        ScanProgressPhase::ScanningArchives => "Scanning archives",
        ScanProgressPhase::BuildingResults => "Building results",
        ScanProgressPhase::Finished => "Finished",
    }
}

fn progress_fraction(phase: ScanProgressPhase, current: usize, total: usize) -> f32 {
    if phase == ScanProgressPhase::Finished {
        return 1.0;
    }
    if total == 0 {
        return 0.0;
    }
    (current as f32 / total as f32).clamp(0.0, 1.0)
}

fn execute_action_button_label(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::QuarantineMove => "Execute Quarantine",
        ActionKind::HardlinkReplace => "Execute Hardlink Replace",
        ActionKind::SymlinkReplace => "Execute Symlink Replace",
    }
}

fn join_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_paths(text: &str) -> Vec<PathBuf> {
    parse_lines(text).into_iter().map(PathBuf::from).collect()
}

fn parse_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

fn parse_optional_path(text: &str) -> Option<PathBuf> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn filtered_group_indices(view: &ResultsViewModel, filter_text: &str) -> Vec<usize> {
    let needle = filter_text.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return (0..view.groups.len()).collect();
    }

    view.groups
        .iter()
        .enumerate()
        .filter_map(|(index, group)| {
            let reason_matches = group.reason.to_ascii_lowercase().contains(&needle);
            let path_matches = group.items.iter().any(|item| {
                item.path
                    .display()
                    .to_string()
                    .to_ascii_lowercase()
                    .contains(&needle)
            });
            (reason_matches || path_matches).then_some(index)
        })
        .collect()
}

fn filtered_report_entries<'a>(
    reports: &'a [dedupe_report_db::StoredReportSummary],
    filter_text: &str,
) -> Vec<&'a dedupe_report_db::StoredReportSummary> {
    let needle = filter_text.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return reports.iter().collect();
    }

    reports
        .iter()
        .filter(|report| {
            report.id.to_string().contains(&needle)
                || report.name.to_ascii_lowercase().contains(&needle)
                || report.mode.to_ascii_lowercase().contains(&needle)
                || report.risk.to_ascii_lowercase().contains(&needle)
        })
        .collect()
}

fn report_source_label(source: &LoadedReportSource) -> String {
    match source {
        LoadedReportSource::Scan => "Source: current scan".to_string(),
        LoadedReportSource::File(path) => format!("Source: report file {}", path.display()),
        LoadedReportSource::ReportDb {
            db_path,
            id,
            name,
            mode,
            risk,
            created_at_unix,
        } => format!(
            "Source: report DB {} | #{} | {} | {} | {} | created {}",
            db_path.display(),
            id,
            name,
            mode,
            risk,
            created_at_unix
        ),
    }
}
