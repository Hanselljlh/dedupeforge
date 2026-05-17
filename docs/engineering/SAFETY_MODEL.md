# Safety model

DedupeForge should prioritize safe review over aggressive cleanup.

## Core invariants

1. Scanning never modifies files.
2. A duplicate group must always retain at least one file.
3. Protected/reference folders cannot be modified by automatic actions.
4. Destructive actions require an explicit action plan.
5. Every action batch must produce a log.
6. Move-to-quarantine should be implemented before permanent delete.
7. Undo support should exist before broad action features are enabled.

## Current MVP safety

The current CLI has no destructive actions.

It only reports duplicate groups and suggested keep items.

## Protected folders

Protected folders represent source-of-truth or archive locations.

Expected behavior:

- protected files can be scanned
- protected files can participate in duplicate groups
- protected files are preferred as keep items
- protected files cannot be selected by auto-delete rules
- protected files should be visibly marked in the UI

## Suggested keep logic

Current MVP rule:

1. keep the first protected file if one exists
2. otherwise keep the first sorted file

Future rules may include:

- prefer newest
- prefer oldest
- prefer shortest path
- prefer longest path
- prefer largest dimensions for images
- prefer lossless over lossy
- prefer RAW over JPEG
- prefer specific root path
- prefer file with richer metadata

Every rule must be explainable.

## Action plan

Before modifying files, DedupeForge should generate an action plan.

An action plan should include:

- source file
- destination or action
- group ID
- match reason
- selected rule
- protected status
- expected file size
- expected hash if available

## Quarantine

Quarantine should be the first cleanup action.

Recommended behavior:

```text
.quarantine/
  2026-05-17T06-00-00Z/
    manifest.json
    files/
      <safe encoded original path>
```

The manifest should map original paths to quarantine paths.

## Undo manifest

The undo manifest should include enough information to restore files.

Fields:

- action batch ID
- timestamp
- original path
- quarantine path
- file size
- hash
- action status
- errors

## Delete behavior

Permanent delete should not be part of the early releases.

When added, it should be advanced-only and disabled by default.

Recycle Bin/trash behavior is platform-dependent and unreliable on some network mounts, so the application must not assume that delete is reversible.

## Hard links and symlinks

Hard link and symlink replacement are advanced features.

Risks:

- hard links require filesystem support
- hard links usually require same filesystem/partition
- symlinks can break when folders are moved
- applications may behave differently with links than with normal files

These actions should require explicit opt-in.
