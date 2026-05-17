# TODO

## Current MVP hardening

- [ ] Add fixture-based tests for exact duplicate grouping.
- [ ] Add tests for protected-folder keep selection.
- [ ] Add tests for JSON and CSV output shape.
- [ ] Add path canonicalization edge-case tests.
- [ ] Add error test for unreadable files.
- [ ] Decide whether zero-byte files should be included by default.

## Cache

- [ ] Create `dedupe-cache` crate.
- [ ] Add SQLite schema.
- [ ] Add cache invalidation rules.
- [ ] Add cache enable/disable CLI option.
- [ ] Add cache location CLI option.

## Actions

- [ ] Create `dedupe-actions` crate.
- [ ] Add action plan model.
- [ ] Add dry-run output.
- [ ] Add quarantine move action.
- [ ] Add undo manifest.
- [ ] Add restore command.

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
