# Repository layout

This file describes the intended repository structure. Some folders are placeholders for future releases.

```text
.
├── Cargo.toml
├── README.md
├── PROJECT_BRIEF.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── SECURITY.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── crates/
│   ├── dedupe-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── fs_walk.rs
│   │       ├── hash.rs
│   │       ├── scan.rs
│   │       └── verify.rs
│   ├── dedupe-cli/
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── dedupe-cache/         # planned SQLite cache crate
│   ├── dedupe-actions/       # planned quarantine/delete/link action crate
│   ├── dedupe-media/         # planned image/video/audio matching crate
│   └── dedupe-gui/           # planned desktop GUI frontend
├── docs/
│   ├── product/
│   │   ├── VISION.md
│   │   ├── FEATURE_MATRIX.md
│   │   ├── ROADMAP.md
│   │   └── USER_WORKFLOWS.md
│   ├── engineering/
│   │   ├── ARCHITECTURE.md
│   │   ├── SCAN_PIPELINE.md
│   │   ├── SAFETY_MODEL.md
│   │   ├── MATCH_ENGINES.md
│   │   ├── CACHE_DESIGN.md
│   │   ├── ACTION_MODEL.md
│   │   └── GUI_PLAN.md
│   └── adr/
│       ├── 0001-rust-core.md
│       ├── 0002-scan-before-action.md
│       └── 0003-fast-hash-options.md
├── examples/
│   ├── commands.md
│   └── sample-output.json
├── scripts/
│   └── dev-check.sh
└── .github/
    ├── workflows/ci.yml
    ├── ISSUE_TEMPLATE/
    │   ├── bug_report.md
    │   ├── feature_request.md
    │   └── match_engine_request.md
    └── pull_request_template.md
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

### dedupe-cache planned

Future crate for persistent scan cache.

Responsibilities:

- SQLite database
- file identity records
- hash cache
- perceptual hash cache
- invalidation logic

### dedupe-actions planned

Future crate for safe file actions.

Responsibilities:

- dry-run action planning
- quarantine moves
- undo manifests
- delete/recycle-bin support
- hard link and symlink replacement

### dedupe-media planned

Future crate for non-exact matching.

Responsibilities:

- image perceptual hashing
- video frame hashing through FFmpeg
- music/audio fingerprinting
- RAW + JPEG pair detection

### dedupe-gui planned

Future desktop application.

Responsibilities:

- source selection
- scan profiles
- result table
- preview panel
- auto-select rules
- action queue
- undo history
