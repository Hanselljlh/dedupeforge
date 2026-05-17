# User workflows

## Workflow 1: Safe exact duplicate report

1. User selects scan folders.
2. User optionally marks archive folders as protected.
3. User runs exact duplicate scan.
4. DedupeForge groups exact duplicates.
5. DedupeForge suggests one keep item per group.
6. User exports JSON or CSV.
7. No files are modified.

## Workflow 2: Compare current folder against archive

1. User selects `/data/current` and `/data/archive`.
2. User marks `/data/archive` as protected.
3. DedupeForge scans both.
4. Matching files in `/data/current` are suggested as duplicate candidates.
5. Files in `/data/archive` are suggested as keep candidates.
6. User reviews before any action.

## Workflow 3: Future quarantine cleanup

1. User scans folders.
2. User reviews duplicate groups.
3. User applies auto-select rules.
4. DedupeForge creates an action plan.
5. User reviews the action plan.
6. DedupeForge moves selected files to quarantine.
7. DedupeForge writes an undo manifest.
8. User can restore files from the manifest.

## Workflow 4: Future photo cleanup

1. User selects photo folders.
2. User chooses similar image mode.
3. User chooses threshold and whether rotation/flip matching is enabled.
4. DedupeForge generates image hashes and caches them.
5. User reviews groups with thumbnails and metadata.
6. User chooses keep files based on path, date, dimensions, and file type.
7. User moves selected duplicates to quarantine.

## Workflow 5: Future NAS conservative mode

1. User selects network-mounted paths.
2. DedupeForge detects or is told that paths are network shares.
3. DedupeForge disables risky actions by default.
4. DedupeForge warns that recycle-bin behavior may not be available.
5. DedupeForge prefers report/export/quarantine over direct delete.
