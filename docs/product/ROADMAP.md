# Roadmap

Current `main` status: **Phases 0-11 are implemented in first-pass form** and released as `v0.1.0`. The project is now in polish, correctness-hardening, and workflow-improvement mode rather than initial feature scaffolding.

## Phase 0: Repository foundation — complete

Goal: make the project GitHub-ready.

Implemented:

- README
- project brief
- architecture docs
- safety model
- issue templates
- CI workflow
- license files
- contribution guide

## Phase 1: MVP exact duplicate scanner — complete

Goal: reliable dry-run exact duplicate reports.

Implemented:

- recursive scan
- same-size grouping
- partial hash prefilter
- BLAKE3 / XXH3-128 / SHA-256
- optional byte verification
- protected/reference folders
- human/JSON/CSV output
- tests for exact grouping, protected keep selection, output shape, path canonicalization, unreadable files, and zero-byte behavior

Exit criteria status:

- unit and integration tests exist
- output contracts are exercised by CLI tests
- scanning is read-only

## Phase 2: Cache and scan profiles — complete

Goal: make repeated large scans practical.

Implemented:

- SQLite cache crate
- file path and metadata cache records
- optional device/inode identity reuse
- full and partial hash cache entries
- mtime-tolerance invalidation policy
- scan profile config files
- named scan presets
- cache clear/rebuild controls

Exit criteria status:

- repeated exact scans can skip unchanged files
- image/video/audio modes can reuse cached fingerprints
- cache can be cleared or rebuilt safely

## Phase 3: Safe action system — complete for exact scans

Goal: allow cleanup without permanent damage.

Implemented:

- action planner
- dry-run action output
- quarantine folder move
- undo manifest
- action log
- restore from manifest
- protected path enforcement
- group invariant: never select every file in a group
- save/load action plans
- execution-time hash revalidation

Exit criteria status:

- no action runs without an explicit action plan
- action planning is exact-mode only
- every action batch is auditable by manifest and log
- quarantine move can be reversed

## Phase 4: GUI prototype — complete in prototype form

Goal: AllDup-style review UI backed by the same engine.

Implemented:

- source selection and scan profile controls
- progress/cancel controls
- result table and group details
- metadata, text/binary, and image preview panel
- keeper override
- result filtering and group pruning
- action queue / action plan controls
- report export/import
- report database browse/store/load workflows

Exit criteria status:

- GUI runs backend scans instead of separate scan logic
- GUI can export and load scan reports
- GUI can build exact-mode action plans and execute/restore action batches

## Phase 5: Similar filename and folder engines — complete in first-pass form

Goal: handle non-content workflows.

Implemented:

- normalized filename matching
- token matching
- edit-distance matching
- folder tree comparison based on file names/sizes
- ignored file patterns for duplicate-folder signatures

Known limitations:

- similar-name results remain high risk and review-only
- duplicate-folder comparison is not a full recursive content-hash equivalence engine
- ignore patterns are not applied to every scan mode yet

## Phase 6: Similar images — complete in first-pass form

Goal: photo cleanup workflows.

Implemented:

- perceptual average hashing
- selectable hash sizes
- Hamming distance threshold
- EXIF date support in reasons
- RAW + JPEG pair detection
- cache-backed image hash reuse

Known limitation:

- the `--image-rotation-invariant` flag exists, but scan grouping does not yet compare all generated rotation/flip variants. True rotation/flip-aware grouping remains planned.

## Phase 7: Video and audio — complete in first-pass form

Goal: large media library cleanup.

Implemented:

- FFmpeg/ffprobe dependency detection
- sampled video frame fingerprints
- duration tolerance
- sampled audio fingerprints
- music metadata comparison in reasons
- cache-backed video/audio fingerprints

Known limitations:

- fingerprints are cryptographic hashes of sampled output, not mature perceptual fingerprints
- long-running media tool calls do not yet have robust timeout controls

## Phase 8: Advanced cleanup features — complete in first-pass form

Goal: power-user cleanup options.

Implemented:

- hard-link replacement action
- symlink replacement action
- ZIP archive scanning
- NAS-conservative preset
- scheduler-friendly report database workflow
- result database browser/storage for CLI and GUI

Known limitations:

- advanced actions are opt-in and exact-mode only
- archive support is ZIP-only
- no built-in scheduler exists; the report DB workflow is suitable for external schedulers

## Phase 9: Utility review modes — complete

Goal: cover common library-hygiene and photo-workflow review tasks.

Implemented:

- explicit RAW + JPEG pair mode
- empty file review mode
- empty folder review mode

Exit criteria status:

- modes reuse the shared scan/report pipeline
- CLI and GUI expose the modes
- risk labels are included in results

## Phase 10: File hygiene modes — complete

Goal: help users clean non-duplicate library problems with the same review-first workflow.

Implemented:

- large-file review mode
- bad-extension detection mode

Exit criteria status:

- modes reuse the shared scan/report pipeline
- CLI and GUI expose the modes
- risk labels distinguish low- and medium-risk results

## Phase 11: Archive hygiene modes — complete for ZIP

Goal: make archive cleanup and archive inspection first-class review workflows.

Implemented:

- duplicate archive-member review mode for ZIP files
- empty-archive review mode for ZIP files
- exact-mode `--scan-archives` support for ZIP members

Exit criteria status:

- archive-focused review modes reuse the shared scan/report pipeline
- CLI and GUI expose the modes
- archive members are treated as review/non-actionable pseudo-items

Known limitations:

- only `.zip` archives are supported
- resource limits for very large archives and zip bombs are not implemented yet

## Current next work

Recommended next milestones:

1. **docs-and-status-cleanup** — keep planning docs synchronized with implemented code and release state.
2. **image-rotation-correctness** — make rotation/flip-aware image grouping actually compare generated variants, or remove/rename the flag.
3. **archive-safety-v2** — add resource limits, streaming where possible, and support for more archive formats.
4. **media-fingerprint-v2** — replace sampled-output cryptographic hashes with more robust perceptual/audio fingerprinting.
5. **hard-link-finder-mode** — add hard-link identity review as a scan mode, separate from hard-link replacement action.
6. **broken-file-mode** — detect corrupt/unreadable media and document review semantics.
7. **release-polish** — formalize changelog entries, release notes, and CI status for each published version.
