# Development plan

Current `main` status: DedupeForge has implemented the original scanner, cache, action, GUI, similarity, hygiene, archive, and report-database roadmap in first-pass form. The next work should focus on correctness, documentation, polish, and tightening safety gaps found during code review.

## Completed milestones

### repo-foundation

- documentation scaffold
- CI workflow
- license files
- issue templates
- contribution guide

### exact-scan-mvp

- exact scan works
- tests exist for grouping, keep selection, outputs, unreadable files, and zero-byte behavior
- JSON/CSV/human output is available
- scans are read-only

### cache-v1

- SQLite cache crate exists
- partial and full hashes can be reused
- image/video/audio fingerprints can be cached
- cache invalidation checks size and modified time
- identity-based reuse is supported when available

### action-plan-v1 / quarantine-v1

- dry-run action planner exists
- validation rejects unsafe plans
- quarantine move exists
- undo manifest and action log exist
- restore command exists
- saved/loaded action plans exist
- hard-link and symlink replacement actions exist as opt-in advanced actions

### gui-prototype

- GUI can select folders and configure scans
- GUI can run scans with progress/cancel controls
- GUI can review grouped results
- GUI can filter/prune groups and override keep items
- GUI can build and execute exact-mode action plans
- GUI can export/import reports
- GUI can browse report database entries

### match-engine expansion

- similar names
- similar images
- RAW + JPEG pairs
- similar videos
- similar audio
- duplicate folders
- empty files/folders
- large files
- bad extensions
- duplicate ZIP archive members
- empty ZIP archives

### release packaging

- Windows release packaging script exists
- `v0.1.0` release includes Windows CLI and GUI assets

## Current priority backlog

1. **Fix PR #18 CI before merge**
   - PR #18 is draft/open and currently fails Clippy in CI.
   - Do not document its scan-setup/review-workspace UX as merged until CI passes and it lands.

2. **Close or refresh stale PR #1**
   - PR #1 is draft/open and conflicted.
   - Phase 1 functionality appears represented on `main`, so the PR should be reconciled or closed.

3. **Rotation-aware image correctness**
   - `--image-rotation-invariant` is exposed, but scan grouping does not yet compare all generated variants.
   - Either implement true variant comparison or rename/remove the flag.

4. **Quarantine filename collision safety**
   - Current quarantine destination names are lossy sanitized paths and can collide.
   - Use unique IDs, hashes, or manifest-safe paths to prevent overwrites.

5. **Restore verification for link actions**
   - Before removing an existing path during restore, verify it is the link DedupeForge created.

6. **Archive safety and format support**
   - Add resource limits for ZIP members.
   - Avoid reading huge archive members into memory where possible.
   - Add 7z/rar/tar support only after safety limits are defined.

7. **Media fingerprint quality**
   - Replace sampled-output cryptographic hashes with stronger perceptual video and acoustic fingerprints.
   - Add FFmpeg/ffprobe timeouts.

8. **Ignore pattern consistency**
   - `--ignore-pattern` is accepted globally but only applies to duplicate-folder signatures today.
   - Decide whether to apply it to all file-collection modes or document it as duplicate-folder-only.

9. **Report DB schema polish**
   - Add schema versioning/migrations.
   - Store scan mode labels in CLI/doc kebab-case instead of debug-lowercase forms.

10. **Additional planned modes**
    - broken-file review mode
    - hard-link finder scan mode
    - richer video/audio previews
    - full recursive content-hash folder comparison

## Verification expectation

Before merging future code changes:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

This environment did not have `cargo` installed during the documentation refresh, so local Rust verification must run on a Rust-enabled machine or through GitHub Actions.
