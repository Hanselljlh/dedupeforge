# DedupeForge

DedupeForge is a duplicate-file investigation tool that combines three ideas:

- an **AllDup-style review workflow** with detailed filters, previews, groups, and actions
- a **Czkawka-style fast backend** with modern hashes, cacheable scans, and automation-friendly design
- a **dupeGuru-style safety model** where every duplicate group has a protected reference concept and a clear keep/delete decision

The current repository is an MVP backend and CLI. It is intentionally non-destructive. It finds exact duplicate files and reports them; it does not delete, move, hard-link, or modify files yet.

## Project status

Current stage: **Phase 7 complete through video and audio**

Implemented:

- recursive file scan
- same-size grouping before hashing
- partial hash prefilter
- full hash confirmation
- hash choices: BLAKE3, XXH3 128-bit, SHA-256
- optional byte-by-byte verification
- protected/reference folders
- suggested keep item per group
- SQLite cache with reusable hash lookups
- scan profiles and named presets
- dry-run action plan generation
- selectable keep-rule action planning
- saved and loaded action plans
- quarantine move execution
- undo manifest writing
- action log writing
- restore from manifest
- similar filename matching
- duplicate folder matching
- similar image matching
- RAW + JPEG pair detection
- EXIF-aware image matching reasons
- cached perceptual image hashes
- similar video matching
- similar audio matching
- FFmpeg/ffprobe dependency detection for media scans
- cache-backed video and audio fingerprints
- ignored file patterns for non-content scans
- GUI session/controller crate scaffold
- `egui` desktop prototype shell
- human, JSON, and CSV output

Not implemented yet:

- archive scanning

## Why this project exists

Existing duplicate tools are useful, but each one emphasizes a different part of the workflow.

DedupeForge aims to combine:

- fast exact scans for large local/NAS collections
- flexible matching strategies
- a review-first interface
- safe defaults
- repeatable automation
- clear logs before any destructive action

The long-term goal is not just to delete duplicates. The goal is to help a user understand why files matched, decide what should be kept, and perform reversible cleanup actions safely.

## Repository layout

```text
.
|-- crates/
|   |-- dedupe-core/          # reusable scan engine library
|   `-- dedupe-cli/           # CLI frontend
|-- docs/
|   |-- product/              # vision, feature matrix, roadmap
|   |-- engineering/          # architecture, safety, scan pipeline
|   `-- adr/                  # architecture decision records
|-- examples/                 # example commands and output shapes
|-- scripts/                  # local development helpers
`-- .github/                  # CI, issue templates, PR template
```

See [docs/REPO_LAYOUT.md](docs/REPO_LAYOUT.md) for the full planned structure.

## Install requirements

- Rust stable toolchain
- Cargo

On Debian/Ubuntu:

```bash
sudo nala install -y curl build-essential pkg-config libssl-dev unzip
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

If you do not use `nala`, replace `nala` with `apt`.

## Build

```bash
cargo build --release
```

Windows workspace note:

- on Windows with the GNU Rust toolchain, use `scripts/test-windows.ps1` for a ready-made test command that sets the expected `HOME`, `CARGO_TARGET_DIR`, linker, and MSYS2 paths
- the script routes build artifacts to `C:\dedupeforge-target`, which avoids GNU linker failures when the repo path contains spaces

## Run examples

Scan one folder:

```bash
cargo run --release --bin dedupeforge -- /path/to/folder
```

Scan multiple folders and protect one archive folder:

```bash
cargo run --release --bin dedupeforge -- /data/current /data/archive --protected /data/archive
```

Use XXH3 128-bit:

```bash
cargo run --release --bin dedupeforge -- /data/photos --hash xxh3-128
```

Use byte-by-byte verification after hashing:

```bash
cargo run --release --bin dedupeforge -- /data/photos --byte-verify
```

Export JSON:

```bash
cargo run --release --bin dedupeforge -- /data/photos --output json > duplicates.json
```

Export CSV:

```bash
cargo run --release --bin dedupeforge -- /data/photos --output csv > duplicates.csv
```

Run similar filename matching:

```bash
cargo run --release --bin dedupeforge -- /data/photos --mode similar-names
```

Run duplicate folder matching with ignored patterns:

```bash
cargo run --release --bin dedupeforge -- /data/library --mode duplicate-folders --ignore-pattern "*.tmp" --ignore-pattern "*.bak"
```

Run similar image matching:

```bash
cargo run --release --bin dedupeforge -- /data/photos --mode similar-images --image-hamming-threshold 12
```

Run rotation-aware similar image matching with a larger perceptual hash:

```bash
cargo run --release --bin dedupeforge -- /data/photos --mode similar-images --image-hash-size 16 --image-rotation-invariant
```

Run similar video matching:

```bash
cargo run --release --bin dedupeforge -- /data/videos --mode similar-videos --media-duration-tolerance-secs 2
```

Run similar audio matching:

```bash
cargo run --release --bin dedupeforge -- /data/music --mode similar-audio --media-duration-tolerance-secs 2
```

Use the network-tolerant cache preset for cross-system scans:

```bash
cargo run --release --bin dedupeforge -- /data/a /data/b --preset network-tolerant
```

This preset enables cache reuse and allows small modified-time drift during cache lookup.

Use explicit cache controls:

```bash
cargo run --release --bin dedupeforge -- /data/photos --cache --cache-path .dedupeforge-cache.sqlite3
```

```bash
cargo run --release --bin dedupeforge -- /data/photos --cache --rebuild-cache
```

```bash
cargo run --release --bin dedupeforge -- /data/photos --clear-cache
```

Generate a dry-run quarantine action plan:

```bash
cargo run --release --bin dedupeforge -- /data/photos --action-plan --output json
```

Choose a different keep rule when building the plan:

```bash
cargo run --release --bin dedupeforge -- /data/photos --action-plan --selection-rule keep-newest
```

Save the generated plan to disk:

```bash
cargo run --release --bin dedupeforge -- /data/photos --action-plan --save-action-plan plan.json --output json
```

Load a previously saved plan:

```bash
cargo run --release --bin dedupeforge -- --load-action-plan plan.json --output json
```

Execute a previously saved plan:

```bash
cargo run --release --bin dedupeforge -- --load-action-plan plan.json --execute-action-plan --quarantine-root .quarantine
```

Validate the generated plan and fail if invariants are broken:

```bash
cargo run --release --bin dedupeforge -- /data/photos --action-plan --validate-action-plan
```

Execute the quarantine move plan and write a manifest:

```bash
cargo run --release --bin dedupeforge -- /data/photos --action-plan --execute-action-plan --quarantine-root .quarantine
```

Restore files from a quarantine manifest:

```bash
cargo run --release --bin dedupeforge -- --restore-manifest .quarantine/1234567890/manifest.json
```

Cache notes:

- `--clear-cache` removes the cache file and exits
- `--rebuild-cache` removes the cache file first, then runs the scan and repopulates it
- default cache path is `.dedupeforge-cache.sqlite3`
- `network-tolerant` enables cache plus a `2` second modified-time tolerance
- CLI flags still override profile and preset defaults

Example profiles are available in [examples/profiles](C:/Users/Shadowed/Documents/New%20project%204/dedupeforge/examples/profiles):

- [local-fast.json](C:/Users/Shadowed/Documents/New%20project%204/dedupeforge/examples/profiles/local-fast.json)
- [network-tolerant.json](C:/Users/Shadowed/Documents/New%20project%204/dedupeforge/examples/profiles/network-tolerant.json)
- [archive-verify.json](C:/Users/Shadowed/Documents/New%20project%204/dedupeforge/examples/profiles/archive-verify.json)

You can load one with:

```bash
cargo run --release --bin dedupeforge -- /data/photos --profile examples/profiles/network-tolerant.json
```

Run tests on Windows:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\test-windows.ps1
```

## Safety behavior

DedupeForge is designed around conservative cleanup.

Current MVP behavior:

- no destructive file operations exist
- protected folders are preferred as keep candidates
- every duplicate group gets exactly one suggested keep item
- exact duplicate groups are based on same size plus same full hash
- optional byte-by-byte verification can be enabled
- zero-byte files are excluded by default through `--min-size 1`

Current action behavior:

- dry-run action plans are available
- move-to-quarantine is implemented
- undo manifests are written for quarantine batches
- action logs are written for quarantine batches
- restore from manifest is implemented
- protected/reference paths cannot be selected for automated quarantine moves
- the planner prevents selecting every file in a duplicate group

See [docs/engineering/SAFETY_MODEL.md](docs/engineering/SAFETY_MODEL.md).

Current similarity behavior:

- `--mode similar-names` is explainable but high risk and should be reviewed manually
- `--mode similar-images` uses perceptual hashing and is high risk
- `--mode similar-videos` uses sampled frame fingerprints and is high risk
- `--mode similar-audio` uses sampled audio fingerprints and is high risk
- `--mode duplicate-folders` uses file-tree overlap and is medium risk
- thresholds are tunable with `--name-similarity-threshold` and `--folder-similarity-threshold`
- image similarity is tunable with `--image-hash-size` and `--image-hamming-threshold`
- image mode can use cache-backed perceptual hashes
- video and audio modes can use cache-backed fingerprints
- image mode can use rotation/flip-aware slower matching with `--image-rotation-invariant`
- video and audio modes depend on `ffmpeg` and `ffprobe`, and report a clear dependency error when either is unavailable
- video and audio matching use `--media-duration-tolerance-secs` to constrain duration drift
- RAW + JPEG pairs are detected by normalized basename matching
- ignored noise files can be excluded with `--ignore-pattern`

Current GUI status:

- `dedupe-gui` exists as a GUI-facing session/controller crate
- it can run scans, build action plans, execute quarantine, restore manifests, and save/load GUI session state
- an `egui`/`eframe` native desktop prototype is attached for scan setup, grouped results review, action planning, quarantine execution, and manifest restore
- scan reports can be exported and reopened from the GUI
- the results side panel now includes metadata and inline text/binary preview support
- the GUI supports `similar-images` mode and shows an explicit false-positive warning for image matches
- the GUI now exposes `similar-videos` and `similar-audio` modes and shows the same warning for media similarity scans

## Planned product modes

- Exact Duplicates
- Similar Names
- Similar Images
- RAW + JPEG Pairs
- Similar Videos
- Similar Music
- Duplicate Folders
- Empty Files
- Empty Folders
- Broken Files
- Bad Extensions
- Large Files
- Hard Link Finder
- Archive Scanner

See [docs/product/ROADMAP.md](docs/product/ROADMAP.md).

## License

This repository is licensed under either:

- MIT License
- Apache License, Version 2.0

at your option.

Do not copy GPL-licensed code from other duplicate-finder projects unless the project license is intentionally changed and the obligations are understood. DedupeForge should use other applications as behavioral references, not as source-code sources.
