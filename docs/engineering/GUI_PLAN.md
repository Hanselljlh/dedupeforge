# GUI plan

Status: implemented in first prototype form.

Current implementation:

- `dedupe-gui` crate exists
- GUI session state is serializable
- controller APIs can run scans and action flows through the shared backend
- view-model shaping exists for grouped result presentation
- `egui`/`eframe` is the chosen native toolkit for the first prototype
- source selection, scan profile controls, grouped results, and action queue controls are wired into the prototype window
- report export/import is wired into the prototype window
- preview panel exists for metadata plus inline text/binary preview
- similar image mode is wired into the prototype window with an explicit false-positive warning
- similar video and similar audio modes are wired into the prototype window with the same false-positive warning
- richer media-specific preview is still pending

The GUI should provide an AllDup-style review workflow while using the Rust backend.

## Main screens

### 1. Start screen

- recent scan profiles
- new scan button
- open saved report
- open cache/settings

### 2. Source selection

- include paths
- exclude paths
- protected/reference paths
- hidden/system file behavior
- network/NAS conservative mode

### 3. Scan profile

- scan mode
- hash algorithm
- partial hash size
- byte verification
- min/max file size
- filename filters
- media-specific options later

### 4. Progress screen

- files discovered
- files hashed
- candidate groups
- errors
- elapsed time
- cancel/pause if possible

### 5. Results screen

Primary view:

- grouped duplicate table
- file path
- filename
- size
- modified date
- protected marker
- suggested keep marker
- match reason
- hash or score

Side panel:

- preview
- metadata
- actions
- notes/warnings

### 6. Auto-select rules

Examples:

- select unprotected duplicates
- keep newest
- keep oldest
- keep shortest path
- keep largest image dimensions
- keep files under preferred folder
- select files outside protected folders

### 7. Action queue

- selected files
- planned action
- validation results
- dry-run summary
- execute button

### 8. History/undo

- action batches
- manifests
- restore actions

## Technology options

Current prototype path:

- Rust backend remains source of truth
- `egui`/`eframe` native frontend for the first prototype
- iterate on workflow completeness before deciding whether a later Tauri front-end is still needed

Alternative paths:

- Qt frontend
- egui frontend
- native Windows UI later if Windows-only features become central

## GUI design principle

Do not hide match reasons.

Every result should answer:

- why these files are grouped
- which one is suggested to keep
- whether any file is protected
- what action will happen
- whether the action can be undone
