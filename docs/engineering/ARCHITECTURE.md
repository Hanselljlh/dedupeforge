# Architecture

DedupeForge is built as a reusable backend with multiple frontends and supporting crates.

```text
                +----------------------+
                |      dedupe-gui      |
                |   desktop frontend   |
                +----------+-----------+
                           |
                +----------v-----------+
                |      dedupe-cli      |
                |     CLI frontend     |
                +----------+-----------+
                           |
                +----------v-----------+
                |     dedupe-core      |
                | scan/group/hash/report|
                +----+-----------+-----+
                     |           |
        +------------+---+   +---+-------------+
        |                |   |                 |
+-------v--------+ +-----v-------+ +-----------v------+
|  dedupe-cache  | | dedupe-actions| |  dedupe-media   |
| SQLite hashes  | | safe actions  | | media matching  |
+----------------+ +---------------+ +-----------------+
                           |
                +----------v-----------+
                |   dedupe-report-db   |
                |  stored scan reports |
                +----------------------+
```

## Current design

The current prototype workspace has multiple active crates:

- `dedupe-core`: backend scan, grouping, hashing, and report engine
- `dedupe-cli`: CLI frontend and report/action-plan entrypoint
- `dedupe-cache`: SQLite-backed reusable hash and fingerprint cache
- `dedupe-actions`: dry-run planning, quarantine execution, restore, and advanced link actions
- `dedupe-media`: image, video, and audio similarity helpers
- `dedupe-report-db`: stored scan report database
- `dedupe-gui`: GUI controller plus desktop `egui` prototype shell

The core library owns:

- file walking
- metadata collection
- hash calculation
- grouping
- byte verification
- duplicate report generation

The CLI owns:

- argument parsing
- config creation
- output formatting

Supporting crates own:

- cache persistence and cache lookup policy
- action planning, validation, manifests, and reversible execution
- similarity and fingerprint helpers for non-exact modes
- stored report persistence and browsing
- GUI session state, previews, and report/action orchestration

## Design rules

### 1. The GUI must not become the engine

The GUI should call backend APIs and display results. It should not implement separate scan logic.

### 2. Scan output should be serializable

Scan results should be usable by:

- GUI
- CLI
- tests
- external tools
- future automation

### 3. Actions should consume reports, not raw UI state

Cleanup actions should run from a validated action plan generated from scan results.

### 4. Matching engines should be modular

Each engine should report:

- match type
- confidence or score where applicable
- exact reason
- engine-specific metadata
- false-positive risk level

### 5. Long-running scans should be resumable and reusable

The cache layer should make repeated scans faster and reduce unnecessary rehashing while keeping destructive operations conservative.

## Data flow

```text
ScanConfig
  ->
File collection
  ->
File identity and metadata
  ->
Candidate grouping
  ->
Match engine
  ->
Duplicate or similar groups
  ->
Report
  ->
Review UI or CLI output
  ->
Action plan
  ->
Quarantine, restore, or link action
  ->
Action log + undo manifest
```

## Threading model

The backend parallelizes expensive read and hash operations where practical. UI frontends should receive progress updates through a progress or event API rather than blocking the main UI thread.

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
