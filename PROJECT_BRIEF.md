# Project brief

## Working name

DedupeForge

## One-line description

A safe, fast, cross-platform duplicate-file investigation tool with an AllDup-style review workflow, Czkawka-style scan performance, and dupeGuru-style conservative grouping.

## Problem

Duplicate-file tools usually excel in one area and fall short in another:

- some have strong UI controls but slower or older hashing choices
- some have fast engines but a less comfortable review workflow
- some are easy to use but limited for large libraries or automation

Large media and backup collections need all three:

1. fast scanning
2. transparent matching logic
3. safe review and reversible actions

## Primary users

- users cleaning large photo/video/music libraries
- users comparing backup drives, NAS shares, and archive folders
- users who want manual review before cleanup
- technical users who want CLI automation but still want a GUI for final decisions

## Product principles

1. **Never surprise the user.** Every match must show the reason it matched.
2. **Prefer reversible actions.** Move to quarantine before permanent deletion.
3. **Protect references.** Archive/source-of-truth folders should be impossible to delete by accident.
4. **Separate scan from action.** Scanning should not modify files.
5. **Keep the engine independent.** CLI and GUI should call the same core backend.
6. **Cache aggressively but verify safely.** Use metadata and hashes to speed rescans, but invalidate cache when file metadata changes.
7. **Make automation possible.** JSON/CSV output and future action manifests should support scripts.

## MVP scope

The first usable release should include:

- exact duplicate scan
- fast hash options
- protected/reference folders
- result grouping
- suggested keep file
- export to JSON and CSV
- no destructive actions

## First production-grade target

The first production-grade release should include:

- SQLite cache
- dry-run action plan
- quarantine move action
- undo manifest
- GUI result review screen
- exact duplicates only

Similar images, videos, music, and duplicate folders should come after the action model is safe.
