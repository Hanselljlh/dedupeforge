# User workflows

These workflows describe the current `main` branch. Actions remain review-first: scans produce reports, action plans are explicit, and non-exact similarity modes are not automatically actionable.

## Workflow 1: Safe exact duplicate report

1. User selects scan folders.
2. User optionally marks archive/reference folders as protected.
3. User runs exact duplicate scan.
4. DedupeForge groups files by size, partial hash, and full hash.
5. Optional byte verification can confirm matches byte-for-byte.
6. DedupeForge suggests one keep item per group, preferring protected paths.
7. User exports human, JSON, or CSV output.
8. No files are modified during scanning.

## Workflow 2: Compare current folder against archive

1. User selects `/data/current` and `/data/archive`.
2. User marks `/data/archive` as protected.
3. DedupeForge scans both.
4. Matching files in `/data/current` are suggested as duplicate candidates.
5. Files in `/data/archive` are suggested as keep candidates.
6. User reviews before any action.
7. If the scan mode is exact, the user can build an explicit action plan.

## Workflow 3: Current exact-scan quarantine cleanup

1. User runs an exact scan.
2. User reviews duplicate groups.
3. User accepts suggested keeps or chooses a keep rule (`keep-suggested`, `keep-newest`, `keep-oldest`).
4. DedupeForge creates a dry-run action plan.
5. User validates/reviews the action plan.
6. DedupeForge revalidates files before execution.
7. DedupeForge moves selected files to quarantine or performs an opt-in link replacement action.
8. DedupeForge writes an action log and undo manifest.
9. User can restore files from the manifest.

Notes:

- Action planning is exact-mode only.
- Protected items are skipped by the planner.
- Advanced hard-link and symlink replacement actions are opt-in and still quarantine originals first.

## Workflow 4: Current photo review workflow

1. User selects photo folders.
2. User chooses `similar-images` or `raw-jpeg-pairs` mode.
3. User tunes image hash size and Hamming distance threshold.
4. DedupeForge generates image hashes and can cache them.
5. DedupeForge groups similar images or RAW+JPEG companion files.
6. User reviews groups with reasons, metadata, and GUI image thumbnails.
7. User decides manually which files should be kept.

Notes:

- Similar-image results are high risk and review-only.
- Exact-mode action plans do not currently operate on similar-image reports.
- `--image-rotation-invariant` is exposed, but full rotation/flip-aware grouping is still pending.

## Workflow 5: Current NAS/network conservative workflow

1. User selects network-mounted paths.
2. User manually chooses `--preset nas-conservative` or a matching profile.
3. DedupeForge enables cache behavior suitable for repeated network scans.
4. DedupeForge enables byte verification for the NAS-conservative preset.
5. User exports reports or stores them in the report database for later review.
6. User avoids advanced cleanup actions unless filesystem behavior is understood.

Pending improvements:

- automatic network-share detection
- explicit recycle-bin availability warnings
- stronger default disabling/warning of risky actions on network shares

## Workflow 6: Report database workflow

1. User runs a scan with `--report-db reports.sqlite3 --store-report-name <name>`.
2. DedupeForge stores the full scan report JSON plus summary metadata.
3. User lists stored reports from CLI or GUI.
4. User reopens a report by ID for review.
5. External schedulers can repeat scans and store named reports without needing DedupeForge to implement a scheduler.

## Workflow 7: Archive hygiene review

1. User selects folders containing `.zip` archives.
2. User chooses `duplicate-archive-members`, `empty-archives`, or exact scan with `--scan-archives`.
3. DedupeForge reports duplicate ZIP members or empty ZIP archives.
4. User reviews results; archive member pseudo-items are non-actionable by default.

Notes:

- ZIP is the only supported archive format today.
- Broader archive formats and archive resource limits remain planned.
