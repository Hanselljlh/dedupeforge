# Changelog

All notable changes to this project should be documented here.

## Unreleased

### Added

- Rust workspace
- `dedupe-core` crate
- `dedupe-cli` crate
- `dedupe-cache` crate
- `dedupe-actions` crate
- exact duplicate scan pipeline
- BLAKE3, XXH3 128-bit, and SHA-256 hash choices
- partial hash prefilter
- optional byte-by-byte verification
- protected/reference folder handling
- SQLite cache with clear/rebuild controls
- scan profiles and named presets
- dry-run action plans with selectable keep rules
- saved, loaded, and executable action plans
- quarantine execution with manifest writing
- action log writing for quarantine batches
- restore from manifest
- human, JSON, and CSV output
- GitHub-ready documentation and templates

### Not yet implemented

- GUI
- similar image/video/music engines
- added similar filename scan mode with normalized, token, and edit-distance matching
- added duplicate folder scan mode with thresholded file-tree overlap matching
- added ignored file patterns for folder-oriented scans
- added scan mode and match-risk reporting in CLI output
- added `dedupe-gui` crate as the first GUI-facing session/controller layer
- added serializable GUI session state and results view models over the shared backend
- added an `egui`/`eframe` native desktop prototype wired to scan, results, action-plan, quarantine, and restore flows
- added GUI report export/import and a results-side preview panel for metadata and inline text/binary inspection
- added `dedupe-media` crate for perceptual image hashing and EXIF-aware image metadata
- added `similar-images` scan mode with selectable hash size, Hamming threshold, and optional rotation/flip-aware matching
- added RAW + JPEG pair detection by normalized basename
- added cache-backed perceptual image hashes and GUI warnings for false-positive image matches
- added FFmpeg/ffprobe-based `similar-videos` and `similar-audio` scan modes
- added cache-backed sampled video and audio fingerprints plus duration-tolerance controls
- added clear dependency reporting when media tools are unavailable and GUI support for the new media modes
