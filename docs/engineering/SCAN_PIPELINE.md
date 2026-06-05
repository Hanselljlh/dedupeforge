# Scan pipeline

This document describes the current scan pipeline on `main`.

## Exact duplicate pipeline

1. **Collect files**
   - Walk one or more input roots.
   - Ignore hidden files when configured.
   - Mark files under protected roots.
   - Capture size, modified time, and platform file identity when available.

2. **Filter by minimum size**
   - Default `--min-size` is `1`, so zero-byte files are excluded from exact scans by default.
   - Use `--min-size 0` or `--mode empty-files` when zero-byte review is desired.

3. **Group by size**
   - Only same-size groups with two or more files continue.

4. **Partial hash prefilter**
   - Hash the configured prefix length with BLAKE3, XXH3-128, or SHA-256.
   - Cache partial hashes when cache is enabled.

5. **Full hash confirmation**
   - Fully hash remaining candidates.
   - Cache full hashes when cache is enabled.

6. **Optional byte verification**
   - If `--byte-verify` is enabled, split same-hash groups by byte-for-byte equality.

7. **Build report**
   - Sort groups by size.
   - Suggest one keep item per group, preferring protected files.
   - Return errors collected during file walking or verification.

8. **Optional ZIP member scan**
   - If `--scan-archives` is enabled, scan `.zip` members and append duplicate member groups.
   - Archive member pseudo-items are treated as protected/non-actionable review entries.

## Similar and hygiene mode pipeline

Most non-exact modes use the shared file collection and report model, then apply mode-specific grouping:

- `similar-names` — normalized names, token overlap, and edit distance.
- `similar-images` — image average hashes, EXIF date reasons, RAW+JPEG pair checks.
- `raw-jpeg-pairs` — normalized basename RAW/JPEG companion matching.
- `similar-videos` — FFmpeg/ffprobe sampled frame fingerprints and duration tolerance.
- `similar-audio` — FFmpeg/ffprobe sampled audio fingerprints, duration tolerance, and metadata reasons.
- `duplicate-folders` — folder signatures with ignored file patterns.
- `empty-files` — zero-byte file review.
- `empty-folders` — empty directory review.
- `large-files` — files at or above the configured `--min-size`.
- `bad-extensions` — common magic-number/content-extension mismatch review.
- `duplicate-archive-members` — duplicate file members across `.zip` archives.
- `empty-archives` — `.zip` archives with no file members.

## Cache pipeline

The SQLite cache is current, not future. The CLI allows cache options for these modes:

- `exact`
- `similar-images`
- `similar-videos`
- `similar-audio`

Cache keys include:

- canonical path
- optional file identity `(device_id, inode)`
- size
- modified time
- algorithm/fingerprint label
- hash scope (`partial` or `full`)
- bytes hashed for partial hashes

Cache lookup validates size and modified time, with optional mtime tolerance for network/NAS presets.

## Progress and cancellation

The core exposes progress events and a cancellation token. The GUI uses these for progress/cancel controls.

Known limitation: cancellation is checked between major phases and during result collection, but long-running Rayon hash work or FFmpeg/ffprobe calls may not stop immediately.

## Archive limitations

Current archive support is ZIP-only. Resource-limit hardening for very large archives, zip bombs, and additional formats such as 7z/rar/tar remains planned.
