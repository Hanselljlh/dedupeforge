# Development plan

## Immediate next tasks

1. Add tests for current exact duplicate scanner.
2. Add fixture generator for test files.
3. Add stable JSON schema for scan reports.
4. Add SQLite cache crate.
5. Add dry-run action plan crate.
6. Add quarantine move action and undo manifest.
7. Only then begin GUI prototype.

## Suggested GitHub milestones

### Milestone: repo-foundation

- documentation complete
- CI enabled
- license files added
- issue templates added

### Milestone: exact-scan-mvp

- exact scan works
- tests pass
- JSON/CSV output documented
- no destructive actions

### Milestone: cache-v1

- SQLite cache exists
- repeated scan reuses valid hashes
- cache invalidation tested

### Milestone: action-plan-v1

- action planner exists
- dry-run plan exists
- validation rejects unsafe plans

### Milestone: quarantine-v1

- quarantine move action exists
- undo manifest exists
- restore command exists

### Milestone: gui-prototype

- GUI can select folders
- GUI can run exact scan
- GUI can review groups
- GUI can export report

## Development order

Do not start with similar images or video.

The safe order is:

1. exact scan correctness
2. cache correctness
3. action safety
4. GUI review
5. similar matching engines

The reason is simple: similar matching creates false positives. The project should have strong review and action safety before introducing fuzzy results.
