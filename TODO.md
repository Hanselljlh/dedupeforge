# TODO

## Current MVP hardening

- [x] Add fixture-based tests for exact duplicate grouping.
- [x] Add tests for protected-folder keep selection.
- [x] Add tests for JSON and CSV output shape.
- [x] Add path canonicalization edge-case tests.
- [x] Add error test for unreadable files.
- [x] Decide whether zero-byte files should be included by default.

Current decision:

- zero-byte files are excluded by default through `--min-size 1`
- include them explicitly by running with `--min-size 0`

## Cache

- [x] Create `dedupe-cache` crate.
- [x] Add SQLite schema.
- [x] Add cache invalidation rules.
- [x] Add cache enable/disable CLI option.
- [x] Add cache location CLI option.
- [x] Add cache clear/rebuild CLI controls.
- [x] Add named cache/profile presets.
- [x] Add example profile files.
- [x] Improve cache identity beyond path + size + modified time.

## Actions

- [x] Create `dedupe-actions` crate.
- [x] Add action plan model.
- [x] Add dry-run output.
- [x] Add quarantine move action.
- [x] Add undo manifest.
- [x] Add restore command.
- [x] Add selectable keep-rule planning.
- [x] Add saved action-plan output.

## GUI

- [x] Create GUI-facing controller/session crate.
- [x] Choose GUI stack.
- [x] Create source selection screen.
- [x] Create scan profile screen.
- [x] Create result group table.
- [x] Create preview panel.
- [x] Create action queue screen.

## Future match engines

- [x] Similar filename engine.
- [x] Duplicate folder engine.
- [x] Similar image engine.
- [x] RAW + JPEG pair detection.
- [x] Similar video engine.
- [x] Similar music/audio engine.

## Advanced cleanup

- [x] Hard-link replacement.
- [x] Symlink replacement.
- [x] Zip archive scanning.
- [x] NAS-conservative preset.
- [x] Report database browser.
- [x] Scheduler-friendly stored scan reports.

## Utility review modes

- [x] Explicit RAW + JPEG pair mode.
- [x] Empty file review mode.
- [x] Empty folder review mode.

## File hygiene modes

- [x] Large-file review mode.
- [x] Bad-extension detection mode.
