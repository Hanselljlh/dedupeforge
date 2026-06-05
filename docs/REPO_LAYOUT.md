# Repository layout

This file describes the active repository structure on `main`.

```text
.
|-- Cargo.toml
|-- README.md
|-- PROJECT_BRIEF.md
|-- CHANGELOG.md
|-- CONTRIBUTING.md
|-- SECURITY.md
|-- LICENSE-MIT
|-- LICENSE-APACHE
|-- crates/
|   |-- dedupe-core/
|   |   |-- Cargo.toml
|   |   |-- src/
|   |   |   |-- lib.rs
|   |   |   |-- fs_walk.rs
|   |   |   |-- hash.rs
|   |   |   |-- scan.rs
|   |   |   |-- similar.rs
|   |   |   `-- verify.rs
|   |   `-- tests/
|   |       `-- integration.rs
|   |-- dedupe-cli/
|   |   |-- Cargo.toml
|   |   |-- src/main.rs
|   |   `-- tests/cli_output.rs
|   |-- dedupe-cache/
|   |   |-- Cargo.toml
|   |   `-- src/lib.rs
|   |-- dedupe-actions/
|   |   |-- Cargo.toml
|   |   `-- src/lib.rs
|   |-- dedupe-media/
|   |   |-- Cargo.toml
|   |   `-- src/lib.rs
|   |-- dedupe-report-db/
|   |   |-- Cargo.toml
|   |   `-- src/lib.rs
|   `-- dedupe-gui/
|       |-- Cargo.toml
|       `-- src/
|           |-- lib.rs
|           `-- main.rs
|-- docs/
|   |-- product/
|   |   |-- VISION.md
|   |   |-- FEATURE_MATRIX.md
|   |   |-- ROADMAP.md
|   |   `-- USER_WORKFLOWS.md
|   |-- engineering/
|   |   |-- ARCHITECTURE.md
|   |   |-- SCAN_PIPELINE.md
|   |   |-- SAFETY_MODEL.md
|   |   |-- MATCH_ENGINES.md
|   |   |-- CACHE_DESIGN.md
|   |   |-- ACTION_MODEL.md
|   |   |-- DEVELOPMENT_PLAN.md
|   |   `-- GUI_PLAN.md
|   `-- adr/
|       |-- 0001-rust-core.md
|       |-- 0002-scan-before-action.md
|       `-- 0003-fast-hash-options.md
|-- examples/
|   |-- commands.md
|   |-- sample-output.json
|   `-- profiles/
|       |-- archive-verify.json
|       |-- local-fast.json
|       `-- network-tolerant.json
|-- scripts/
|   |-- build-windows-release.ps1
|   |-- dev-check.sh
|   `-- test-windows.ps1
`-- .github/
    |-- workflows/ci.yml
    |-- ISSUE_TEMPLATE/
    |   |-- bug_report.md
    |   |-- feature_request.md
    |   `-- match_engine_request.md
    |-- CODEOWNERS
    |-- pull_request_template.md
```

## Crate responsibilities

### dedupe-core

Reusable backend crate.

Responsibilities:

- filesystem walking
- file metadata collection
- exact duplicate grouping
- hashing and byte verification
- similar-name/image/video/audio grouping
- RAW+JPEG pair review
- folder, utility, file-hygiene, and archive-hygiene modes
- duplicate report generation
- scan progress/cancel API

### dedupe-cli

CLI frontend.

Responsibilities:

- parse command line options
- invoke `dedupe-core`
- print human, JSON, or CSV reports
- trigger cache, report-db, and action-plan workflows
- restore manifests
- load/validate/execute saved action plans

### dedupe-cache

Persistent SQLite scan cache.

Responsibilities:

- file identity records
- partial and full hash cache entries
- perceptual image hash cache entries
- video/audio fingerprint cache entries
- mtime-tolerance invalidation logic

### dedupe-actions

Safe file action crate.

Responsibilities:

- dry-run action planning
- save/load/validate action plans
- quarantine moves
- hard-link replacement
- symlink replacement
- undo manifests
- action logs
- restore from manifest
- execution-time hash revalidation

### dedupe-media

Image/video/audio helper crate.

Responsibilities:

- image perceptual hashing
- image thumbnail/metadata support used by GUI helpers
- EXIF date extraction
- video frame fingerprinting through FFmpeg
- audio sample fingerprinting through FFmpeg
- media metadata probing through ffprobe
- media extension detection

### dedupe-report-db

Persisted scan report database crate.

Responsibilities:

- store scan reports as JSON in SQLite
- list stored report summaries
- load stored reports by ID

### dedupe-gui

Desktop GUI crate.

Responsibilities:

- serializable GUI session state
- scan setup and progress/cancel controls
- result view models
- result filtering/pruning
- keeper override
- report import/export
- report database browsing/storage/loading
- action-plan creation and execution
- manifest restore
- metadata, text/binary, and image preview panel
- `egui`/`eframe` native app shell
