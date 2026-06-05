# TODO

This file records the original roadmap checklist and the current reviewed backlog.

## Current status

The original MVP-through-archive-hygiene checklist is complete on `main` in first-pass form and released as `v0.1.0`.

## Completed MVP hardening

- [x] Add fixture-based tests for exact duplicate grouping.
- [x] Add tests for protected-folder keep selection.
- [x] Add tests for JSON and CSV output shape.
- [x] Add path canonicalization edge-case tests.
- [x] Add error test for unreadable files.
- [x] Decide whether zero-byte files should be included by default.

Current decision:

- zero-byte files are excluded by default through `--min-size 1`
- include them explicitly by running with `--min-size 0`
- use `--mode empty-files` for zero-byte review

## Completed cache work

- [x] Create `dedupe-cache` crate.
- [x] Add SQLite schema.
- [x] Add cache invalidation rules.
- [x] Add cache enable/disable CLI option.
- [x] Add cache location CLI option.
- [x] Add cache clear/rebuild CLI controls.
- [x] Add named cache/profile presets.
- [x] Add example profile files.
- [x] Improve cache identity beyond path + size + modified time.

## Completed action work

- [x] Create `dedupe-actions` crate.
- [x] Add action plan model.
- [x] Add dry-run output.
- [x] Add quarantine move action.
- [x] Add undo manifest.
- [x] Add restore command.
- [x] Add selectable keep-rule planning.
- [x] Add saved action-plan output.
- [x] Add hard-link replacement action.
- [x] Add symlink replacement action.
- [x] Add execution-time hash revalidation.

## Completed GUI work

- [x] Create GUI-facing controller/session crate.
- [x] Choose GUI stack.
- [x] Create source selection screen.
- [x] Create scan profile screen.
- [x] Create result group table.
- [x] Create preview panel.
- [x] Create action queue screen.
- [x] Add scan progress/cancel controls.
- [x] Add result filtering/pruning.
- [x] Add keeper override.
- [x] Add report import/export.
- [x] Add report database browser workflow.
- [x] Add image thumbnail preview.

## Completed match engines

- [x] Similar filename engine.
- [x] Duplicate folder engine.
- [x] Similar image engine.
- [x] RAW + JPEG pair detection.
- [x] Similar video engine.
- [x] Similar music/audio engine.

## Completed advanced cleanup

- [x] Hard-link replacement.
- [x] Symlink replacement.
- [x] Zip archive scanning.
- [x] NAS-conservative preset.
- [x] Report database browser.
- [x] Scheduler-friendly stored scan reports.

## Completed utility review modes

- [x] Explicit RAW + JPEG pair mode.
- [x] Empty file review mode.
- [x] Empty folder review mode.

## Completed file hygiene modes

- [x] Large-file review mode.
- [x] Bad-extension detection mode.

## Completed archive hygiene modes

- [x] Duplicate archive-member review mode.
- [x] Empty-archive review mode.

## Current backlog from code review

- [ ] Fix PR #18 Clippy failure before documenting/merging its workspace UX.
- [ ] Reconcile or close stale/conflicted PR #1.
- [ ] Implement true rotation/flip-aware image grouping or rename/remove the flag.
- [ ] Prevent quarantine destination filename collisions.
- [ ] Verify link-action restore targets before removing existing paths.
- [ ] Add archive resource limits and broader archive formats beyond ZIP.
- [ ] Add FFmpeg/ffprobe timeouts.
- [ ] Improve video/audio fingerprints beyond sampled-output cryptographic hashes.
- [ ] Make `--ignore-pattern` behavior consistent across modes or document it as mode-specific.
- [ ] Add schema versions/migrations for cache and report DB.
- [ ] Normalize report DB mode labels to CLI/doc kebab-case.
- [ ] Add broken-file review mode.
- [ ] Add hard-link finder scan mode.
- [ ] Add richer video/audio previews.
