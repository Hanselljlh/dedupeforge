# DedupeForge

DedupeForge is a duplicate-file investigation tool that combines three ideas:

- an **AllDup-style review workflow** with detailed filters, previews, groups, and actions
- a **Czkawka-style fast backend** with modern hashes, cacheable scans, and automation-friendly design
- a **dupeGuru-style safety model** where every duplicate group has a protected reference concept and a clear keep/delete decision

The current repository is an MVP backend and CLI. It is intentionally non-destructive. It finds exact duplicate files and reports them; it does not delete, move, hard-link, or modify files yet.

## Project status

Current stage: **MVP exact duplicate scanner**

Implemented:

- recursive file scan
- same-size grouping before hashing
- partial hash prefilter
- full hash confirmation
- hash choices: BLAKE3, XXH3 128-bit, SHA-256
- optional byte-by-byte verification
- protected/reference folders
- suggested keep item per group
- human, JSON, and CSV output

Not implemented yet:

- SQLite cache
- quarantine/move actions
- undo manifests
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

- this repository includes [.cargo/config.toml](.cargo/config.toml) to route build artifacts to `C:\dedupeforge-target`, which avoids GNU linker failures when the repo path contains spaces
- on Windows with the GNU Rust toolchain, use `scripts/test-windows.ps1` for a ready-made test command that sets the expected `HOME` and MSYS2 paths

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

Future action behavior:

- dry-run by default
- move-to-quarantine before delete
- undo manifest for every action batch
- never allow every file in a group to be selected for deletion
- protected/reference paths cannot be deleted by automated rules

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
