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
7. **Make automation possible.** JSON/CSV output, saved action plans, report databases, and manifests should support scripts.

## MVP scope — completed

The first usable release scope included:

- exact duplicate scan
- fast hash options
- protected/reference folders
- result grouping
- suggested keep file
- export to JSON and CSV
- scan-only operation by default

## First production-grade target — implemented in first-pass form

The first production-grade target included:

- SQLite cache
- dry-run action plan
- quarantine move action
- undo manifest
- GUI result review screen
- exact-duplicates action planning

## Current implemented expansion

The current `main` branch also includes first-pass implementations for:

- similar images, videos, music/audio, names, and folders
- RAW + JPEG companion review
- empty file/folder review
- large-file and bad-extension review
- ZIP archive-member and empty-archive review
- hard-link and symlink replacement actions for exact-mode plans
- report database workflows for CLI and GUI

## Current known gaps

- broken-file review mode
- hard-link finder scan mode
- archive support beyond ZIP
- true rotation/flip-aware image grouping
- stronger perceptual video/audio fingerprinting
- broader safety hardening around archive resource limits and link-action restore verification
