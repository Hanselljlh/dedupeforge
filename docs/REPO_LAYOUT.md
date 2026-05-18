# Repository layout

This file describes the intended repository structure. Some folders are still placeholders for future expansion, but the major crates below are active today.

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
|   |   `-- src/
|   |       |-- lib.rs
|   |       |-- fs_walk.rs
|   |       |-- hash.rs
|   |       |-- scan.rs
|   |       `-- verify.rs
|   |-- dedupe-cli/
|   |   |-- Cargo.toml
|   |   `-- src/main.rs
|   |-- dedupe-cache/         # SQLite cache crate
|   |-- dedupe-actions/       # quarantine/restore/link action crate
|   |-- dedupe-media/         # image/video/audio matching crate
|   |-- dedupe-report-db/     # stored report database crate
|   `-- dedupe-gui/           # desktop GUI frontend
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
|   |   `-- GUI_PLAN.md
|   `-- adr/
|       |-- 0001-rust-core.md
|       |-- 0002-scan-before-action.md
|       `-- 0003-fast-hash-options.md
|-- examples/
|   |-- commands.md
|   `-- sample-output.json
|-- scripts/
|   |-- dev-check.sh
|   `-- test-windows.ps1
`-- .github/
    |-- workflows/ci.yml
    |-- ISSUE_TEMPLATE/
    |   |-- bug_report.md
    |   |-- feature_request.md
    |   `-- match_engine_request.md
    `-- pull_request_template.md
```

## Crate responsibilities

### dedupe-core

Current reusable backend crate.

Responsibilities:

- filesystem walking
- file metadata collection
- grouping
- hashing
- byte verification
- duplicate report generation

### dedupe-cli

Current CLI frontend.

Responsibilities:

- parse command line options
- invoke `dedupe-core`
- print human, JSON, or CSV output
- trigger report-db and action-plan workflows

### dedupe-cache

Current crate for persistent scan cache.

Responsibilities:

- SQLite database
- file identity records
- hash cache
- perceptual hash and media fingerprint cache support
- invalidation logic

### dedupe-actions

Current crate for safe file actions.

Responsibilities:

- dry-run action planning
- quarantine moves
- undo manifests
- restore from manifest
- hard link and symlink replacement

### dedupe-media

Current crate for non-exact matching.

Responsibilities:

- image perceptual hashing
- video frame hashing through FFmpeg
- music or audio fingerprinting
- RAW + JPEG pair detection

### dedupe-report-db

Current crate for persisted scan reports.

Responsibilities:

- SQLite report storage
- report listing and loading
- scheduler-friendly saved scan history

### dedupe-gui

Current desktop application prototype.

Responsibilities:

- source selection
- scan profiles
- result table
- preview panel
- action queue
- session persistence
- report export and reopen
