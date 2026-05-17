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

- [ ] Choose GUI stack.
- [ ] Create source selection screen.
- [ ] Create scan profile screen.
- [ ] Create result group table.
- [ ] Create preview panel.
- [ ] Create action queue screen.

## Future match engines

- [ ] Similar filename engine.
- [ ] Duplicate folder engine.
- [ ] Similar image engine.
- [ ] RAW + JPEG pair detection.
- [ ] Similar video engine.
- [ ] Similar music/audio engine.
