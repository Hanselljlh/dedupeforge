# Changelog

All notable changes to this project should be documented here.

## Unreleased

### Documentation

- Updated project status documents to match the current implementation through Phase 11/archive hygiene modes.
- Documented known limitations from a full source review, including ZIP-only archive support, first-pass media fingerprints, exact-mode-only action plans, and the pending true rotation/flip-aware image grouping work.
- Added a current backlog for safety and polish follow-up work.

## v0.1.0 - 2026-05-19

### Added

- exact duplicate scan pipeline with same-size grouping, partial hash prefilter, full hash confirmation, and optional byte verification
- BLAKE3, XXH3-128, and SHA-256 hash choices
- protected/reference folder handling and suggested keep selection
- human, JSON, and CSV report output
- SQLite cache crate with reusable partial/full hash lookups and file identity support
- scan profiles and named presets, including network/NAS-oriented presets
- dry-run action plans, selectable keep rules, saved/loaded plans, validation, and execution-time hash revalidation
- quarantine move action with undo manifest, action log, and restore workflow
- opt-in hard-link and symlink replacement actions
- GUI controller crate and `egui` desktop prototype
- GUI scan progress/cancel controls
- GUI result filtering, group pruning, keeper override, and action planning controls
- GUI metadata, text/binary preview, and image thumbnail preview
- GUI report export/import and report database workflows
- similar filename, duplicate folder, similar image, RAW+JPEG pair, similar video, and similar audio scan modes
- cache-backed perceptual image hashes and sampled media fingerprints
- explicit RAW+JPEG pair mode
- empty file and empty folder review modes
- large-file and bad-extension hygiene review modes
- duplicate ZIP archive-member and empty ZIP archive review modes
- ZIP archive member scanning in exact mode through `--scan-archives`
- `dedupe-report-db` crate with SQLite report storage/list/load workflows
- scheduler-friendly report database command flow
- Windows test and release packaging scripts
- Windows `dedupeforge.exe` and `dedupeforge-gui.exe` release assets
