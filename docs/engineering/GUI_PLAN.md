# GUI plan

Status: implemented in first prototype form.

The GUI should provide an AllDup-style review workflow while using the Rust backend as the source of truth.

## Current implementation

- `dedupe-gui` crate exists.
- GUI session state is serializable.
- `egui`/`eframe` is the native toolkit for the first prototype.
- Controller APIs run scans and action flows through the shared backend.
- Scans run on a worker thread with progress polling and cancellation.
- View-model shaping exists for grouped result presentation.
- Source selection and scan profile controls are wired into the prototype window.
- All current scan modes are exposed in the GUI.
- Media-specific scan options are exposed, including image hash size, image threshold, image rotation flag, duration tolerance, and fingerprint distance threshold.
- Grouped results, filtering, group pruning, and keeper override are wired.
- Action plan controls are wired for exact-mode reports.
- Action type selection supports quarantine move, hard-link replacement, and symlink replacement.
- Quarantine execution and manifest restore are wired.
- Report export/import is wired.
- Report database refresh, store, and load workflows are wired.
- Preview panel supports metadata, inline text/binary previews, and image thumbnails.
- Similar image/video/audio modes show false-positive warnings.

## Known GUI limitations

- Action plans are exact-mode only; similarity and hygiene reports are review-only.
- Rich video/audio-specific previews remain pending.
- The image rotation flag is exposed, but true rotation/flip-aware scan grouping is still a backend gap.
- Cancel requests may not interrupt long-running hash/media operations immediately.
- PR #18’s separate scan-setup/review-workspace UX is not merged; it is still a draft PR with failing CI.

## Main screens

### 1. Start / setup screen

- recent or loaded scan profile state
- include paths
- protected/reference paths
- scan mode
- hidden/system file behavior
- network/NAS conservative preset options
- cache controls
- report database path controls

### 2. Scan profile

- hash algorithm
- partial hash size
- byte verification
- min file size
- ignore patterns where supported
- image/media thresholds
- archive scan toggle

### 3. Progress screen

- current phase
- current/total counts when available
- status message
- cancel control

### 4. Results screen

Primary view:

- grouped result table
- file path
- filename
- size
- modified date
- protected marker
- suggested keep marker
- match reason
- hash/score/engine label
- filters and remove-from-review controls

Side panel:

- preview
- metadata
- warnings
- keeper override
- action setup for exact-mode reports

### 5. Auto-select and keeper controls

Implemented/current:

- keep suggested
- keep newest
- keep oldest
- GUI keeper override for exact-mode groups

Possible future rules:

- keep shortest path
- keep largest image dimensions
- keep files under preferred folder
- select files outside protected folders

### 6. Action queue

- selected files
- planned action
- validation results
- dry-run summary
- execute button for exact-mode plans

### 7. History / undo

- manifest path restore
- report DB browse/load
- saved sessions and reports

## GUI design principle

Do not hide match reasons.

Every result should answer:

- why these files are grouped
- which one is suggested to keep
- whether any file is protected
- what action will happen, if an action plan is built
- whether the action can be undone
