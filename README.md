# DedupeForge

DedupeForge is a duplicate-file investigation tool that combines three ideas:

- an **AllDup-style review workflow** with detailed filters, previews, groups, and actions
- a **Czkawka-style fast backend** with modern hashes, cacheable scans, and automation-friendly design
- a **dupeGuru-style safety model** where every duplicate group has a protected reference concept and a clear keep/delete decision

The current repository is an MVP backend and CLI. It is intentionally non-destructive. It finds exact duplicate files and reports them; it does not delete, move, hard-link, or modify files yet.

## Project status

Current stage: **Phase 3 complete, GUI prototype next**

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
- human, JSON, and CSV output

Not implemented yet:

- GUI
- similar image matching
- similar video matching
- music/audio matching
- duplicate folder matching
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
