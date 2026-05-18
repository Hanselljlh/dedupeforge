# Action model

Status: implemented in first safe-action form.

Actions must be separate from scanning.

## Action flow

```text
Scan report
  ↓
Selection rules
  ↓
Action plan
  ↓
Validation
  ↓
Dry-run output
  ↓
User approval
  ↓
Execution
  ↓
Action log + undo manifest
```

## Supported actions by priority

### Phase 1 actions

- export report
- save action plan

### Current implemented actions

- move selected files to quarantine
- restore from quarantine

### Phase 3 actions

- move selected files to another folder
- copy selected files
- rename selected files

### Phase 4 actions

- send to trash/recycle bin
- permanent delete
- replace with hard link
- replace with symlink

## Action plan validation

Before execution, validate:

- every selected file still exists
- selected file size still matches report
- selected file hash still matches report if required
- no selected file is protected
- every group retains at least one unselected keep item
- destination paths are writable
- no destination collision exists unless policy handles it

## Action result statuses

- planned
- skipped
- completed
- failed
- restored

## Manifest format

Current `manifest.json` shape:

```json
{
  "version": 1,
  "batch_id": "2026-05-17T06-00-00Z",
  "created_at": "2026-05-17T06:00:00Z",
  "action": "quarantine_move",
  "items": [
    {
      "group_id": "group-0001",
      "original_path": "/data/current/photo.jpg",
      "quarantine_path": "/data/.dedupeforge-quarantine/2026-05-17/photo.jpg",
      "size": 123456,
      "hash_algorithm": "blake3",
      "hash": "...",
      "status": "completed"
    }
  ]
}
```

## Action log

Each quarantine batch also writes a plain-text `action.log` next to `manifest.json`.

This log is intended to provide a quick human-auditable trail without requiring JSON parsing.
