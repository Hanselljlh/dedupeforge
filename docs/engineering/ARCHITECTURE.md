# Architecture

DedupeForge should be built as a reusable backend with multiple frontends.

```text
                ┌──────────────────────┐
                │      dedupe-gui       │
                │  desktop frontend     │
                └──────────┬───────────┘
                           │
                ┌──────────▼───────────┐
                │      dedupe-cli       │
                │    CLI frontend       │
                └──────────┬───────────┘
                           │
                ┌──────────▼───────────┐
                │     dedupe-core       │
                │ scan/group/hash/report│
                └──────────┬───────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
┌───────▼───────┐  ┌───────▼────────┐ ┌───────▼────────┐
│ dedupe-cache  │  │ dedupe-actions │ │ dedupe-media   │
│ planned SQLite│  │ planned actions│ │ planned media  │
└───────────────┘  └────────────────┘ └────────────────┘
```

## Current design

The current MVP has two crates:

- `dedupe-core`: backend library
- `dedupe-cli`: CLI frontend

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

## Design rules

### 1. The GUI must not become the engine

The GUI should call backend APIs and display results. It should not implement separate scan logic.

### 2. Scan output should be serializable

Scan results should be usable by:

- GUI
- CLI
- tests
- external tools
- future action planner

### 3. Actions should consume reports, not raw UI state

Future cleanup actions should run from a validated action plan generated from scan results.

### 4. Matching engines should be modular

Each engine should report:

- match type
- confidence/score where applicable
- exact reason
- engine-specific metadata
- false-positive risk level

### 5. Long-running scans should be resumable later

The future cache layer should make repeated scans faster and support interrupted scan recovery where practical.

## Planned data flow

```text
ScanConfig
  ↓
File collection
  ↓
File identity and metadata
  ↓
Candidate grouping
  ↓
Match engine
  ↓
Duplicate/Similar groups
  ↓
Report
  ↓
Review UI or CLI output
  ↓
Action plan
  ↓
Quarantine/delete/link action
  ↓
Action log + undo manifest
```

## Threading model

The backend should parallelize expensive read/hash operations. UI frontends should receive progress updates through a progress/event API rather than blocking the main UI thread.

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
