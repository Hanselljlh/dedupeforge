# Architecture

DedupeForge is built as a reusable backend with multiple frontends and supporting crates.

```text
                  +----------------------+
                  |      dedupe-gui      |
                  |   desktop frontend   |
                  +----------+-----------+
                             |
                  +----------v-----------+
                  |     GUI controller   |
                  | session/review state |
                  +----------+-----------+
                             |
+----------------------------+----------------------------+
|                                                         |
|                +----------------------+                 |
|                |      dedupe-cli      |                 |
|                |     CLI frontend     |                 |
|                +----------+-----------+                 |
|                           |                             |
+---------------------------v-----------------------------+
                            |
                 +----------v-----------+
                 |     dedupe-core      |
                 | scan/group/hash/report|
                 +----+-----------+-----+
                      |           |
         +------------+---+   +---+-------------+
         |                |   |                 |
+--------v-------+ +------v------+ +------------v-----+
|  dedupe-cache  | | dedupe-actions| |  dedupe-media   |
| SQLite hashes  | | safe actions  | | media matching  |
+----------------+ +------+-------+ +------------------+
                          |
                 +--------v-------------+
                 |   dedupe-report-db   |
                 |  stored scan reports |
                 +----------------------+
```

## Current active crates

- `dedupe-core`: backend scan, grouping, hashing, progress, and report engine
- `dedupe-cli`: CLI frontend and report/action/cache/report-db entrypoint
- `dedupe-cache`: SQLite-backed reusable hash and fingerprint cache
- `dedupe-actions`: dry-run planning, quarantine execution, restore, and advanced link actions
- `dedupe-media`: image, video, and audio similarity helpers
- `dedupe-report-db`: stored scan report database
- `dedupe-gui`: GUI controller plus desktop `egui` prototype shell

## Core responsibilities

The core library owns:

- file walking
- metadata and file identity collection
- exact duplicate grouping
- partial and full hashing
- byte verification
- similar-name/image/video/audio grouping
- folder, utility, file-hygiene, and archive-hygiene modes
- duplicate/similar report generation
- scan progress and cancellation API

## CLI responsibilities

The CLI owns:

- argument parsing
- config/profile/preset resolution
- cache command dispatch
- output formatting for human, JSON, and CSV reports
- action-plan generation/loading/validation/execution
- manifest restore
- report database store/list/load commands

## GUI responsibilities

The GUI owns:

- serializable session state
- scan setup controls
- worker-thread scan orchestration with progress/cancel
- result view models
- result filtering and review pruning
- keeper override for exact-mode reports
- action-plan setup/execution for exact-mode reports
- manifest restore
- report export/import
- report database refresh/store/load workflows
- metadata, text/binary, and image previews

## Supporting crate responsibilities

- `dedupe-cache`: cache persistence and lookup/invalidation policy
- `dedupe-actions`: action planning, validation, manifests, logs, and reversible execution
- `dedupe-media`: media/image analysis and FFmpeg/ffprobe wrappers
- `dedupe-report-db`: report JSON persistence and browsing

## Design rules

### 1. The GUI must not become the engine

The GUI calls backend APIs and displays results. It must not implement separate scan logic.

### 2. Scan output should be serializable

Scan results are usable by:

- GUI
- CLI
- tests
- external tools
- report database storage
- future automation

### 3. Actions consume reports, not raw UI state

Cleanup actions run from validated action plans generated from exact scan reports.

### 4. Matching engines are modular and explainable

Each engine should report:

- match type or engine label
- confidence/score/hash where applicable
- exact reason
- engine-specific metadata where available
- false-positive risk level

### 5. Repeated scans should be reusable

The cache layer reduces unnecessary rehashing/fingerprinting while keeping destructive operations conservative.

## Data flow

```text
ScanConfig
  -> File collection
  -> File identity and metadata
  -> Match engine
  -> Duplicate or similar groups
  -> ScanReport
  -> CLI output, GUI review, or report DB storage
  -> Exact-mode action plan, if requested
  -> Quarantine, restore, hard-link, or symlink action
  -> Action log + undo manifest
```

## Threading model

The backend parallelizes expensive read and hash operations where practical. The GUI runs scans in a worker thread and receives progress events through the controller.

Known limitation: cancellation may not interrupt long-running hash/media operations immediately.

## Error model

A scan should continue when individual files fail, unless the root scan path itself is invalid.

Per-file errors should be included in the report.

Examples:

- permission denied
- file disappeared during scan
- file changed during scan
- unsupported media file
- FFmpeg failure

## Output stability

JSON output should be treated as an API once the project reaches a stable release. Until then, output may change, but changes should be listed in the changelog.
