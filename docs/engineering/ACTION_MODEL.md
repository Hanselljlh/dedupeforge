# Action model

DedupeForge actions are explicit, reviewable, and reversible where possible. Action planning is currently exact-mode only.

## Current implemented actions

- build dry-run action plans from exact duplicate reports
- save action plans to JSON
- load action plans from JSON
- validate plans against the current filesystem
- execute quarantine moves
- execute hard-link replacement, opt-in
- execute symlink replacement, opt-in
- write undo manifests
- write action logs
- restore files from quarantine manifests
- revalidate file hashes before destructive execution

## Action flow

1. User runs an exact scan.
2. DedupeForge suggests one keep item per duplicate group.
3. User chooses a selection rule or overrides keep choices in the GUI.
4. DedupeForge builds a dry-run action plan.
5. User reviews/validates the plan.
6. DedupeForge revalidates planned files before execution.
7. DedupeForge executes the chosen action.
8. DedupeForge writes `manifest.json` and `action.log`.
9. User can restore from the manifest.

## Selection rules

- `keep-suggested`
- `keep-newest`
- `keep-oldest`

## Action kinds

### `quarantine_move`

Moves selected duplicate files into a quarantine batch directory.

### `hardlink_replace`

Moves the selected duplicate to quarantine, then creates a hard link at the original path pointing to the kept file.

Safety checks:

- replacement target exists
- target and source are on the same filesystem
- planned item and target still match the planned hash before execution

### `symlink_replace`

Moves the selected duplicate to quarantine, then creates a symlink at the original path pointing to the kept file.

Safety checks:

- replacement target exists
- symlink support is probed before execution
- planned item and target still match the planned hash before execution

## Manifest shape

Current manifests include:

```json
{
  "version": 1,
  "batch_id": "1234567890",
  "created_at_unix": 1234567890,
  "action": "quarantine_move",
  "quarantine_root": ".quarantine/1234567890",
  "items": [
    {
      "group_id": "group-0000",
      "original_path": "/data/current/photo-copy.jpg",
      "quarantine_path": ".quarantine/1234567890/files/_data_current_photo-copy.jpg",
      "size": 12345,
      "hash_algorithm": "blake3",
      "hash": "...",
      "replacement_target": null,
      "status": "completed",
      "error": null
    }
  ]
}
```

For link replacement actions, `replacement_target` records the kept file used for the hard link or symlink.

## Non-actionable reports

The planner rejects every non-exact scan mode. Similarity and hygiene reports are for review only until a future workflow defines safe action semantics for them.

## Future actions / improvements

Still planned or not implemented:

- permanent delete
- recycle-bin/trash integration
- copy/rename actions
- hard-link finder as a scan mode
- stronger quarantine filename collision prevention
- stronger restore verification for link actions
- action support for carefully constrained non-exact review modes, if safety can be proven
