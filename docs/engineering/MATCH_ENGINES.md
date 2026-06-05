# Match engines

A match engine takes indexed files and produces groups of related files. This document reflects the current `main` branch after source review.

## Required output for every engine

Each group should include:

- items
- match type or engine label
- match reason
- confidence score or hash where applicable
- false-positive risk level on the report
- suggested keep item
- engine-specific metadata where available

## Engine 1: Exact duplicates

Status: implemented.

Rules:

- file sizes must match
- partial hashes must match before full hashing
- full content hashes must match
- optional byte verification may be enabled
- protected roots are preferred as keep candidates

False-positive risk:

- very low with cryptographic hash
- very low with byte verification
- low with fast non-cryptographic hash, but byte verification is recommended before destructive action

## Engine 2: Similar filenames

Status: implemented in first-pass form.

Methods:

- normalized filename comparison
- token overlap
- Levenshtein-style edit distance
- punctuation/separator normalization

False-positive risk:

- high depending on threshold and naming patterns

Notes:

- Results are review-only.
- Threshold tuning is available via `--name-similarity-threshold`.

## Engine 3: Similar images

Status: implemented in first-pass form.

Methods:

- perceptual average hash
- configurable hash size
- Hamming distance threshold
- EXIF date support in match reasons
- RAW + JPEG pair detection by normalized basename
- cache-backed image hash reuse

False-positive risk:

- high for perceptual-hash results
- medium for RAW + JPEG pair review

Known limitation:

- `--image-rotation-invariant` is exposed and changes analysis/cache labeling/reason text, but scan grouping does not yet compare every generated rotation/flip variant. True rotation/flip-aware grouping is still planned.

## Engine 4: Similar videos

Status: implemented in first-pass form.

Methods:

- FFmpeg/ffprobe dependency detection
- duration tolerance
- sampled frame fingerprinting
- Hamming-distance threshold over cached fingerprints

False-positive risk:

- high; depends heavily on sampling strategy and thresholds

Known limitation:

- The current fingerprint is a cryptographic hash of sampled frame output, not a robust perceptual video fingerprint. Small changes may avalanche.

## Engine 5: Similar music/audio

Status: implemented in first-pass form.

Methods:

- FFmpeg/ffprobe dependency detection
- duration tolerance
- sampled audio fingerprinting
- metadata comparison in reasons for title/artist/album
- Hamming-distance threshold over cached fingerprints

False-positive risk:

- high until a stronger acoustic fingerprinting strategy is added

Known limitation:

- The current fingerprint is a cryptographic hash of sampled audio output, not a mature acoustic fingerprint.

## Engine 6: Duplicate folders

Status: implemented in first-pass form.

Methods:

- file-tree signatures per directory
- filename and size based comparison
- ignored file patterns
- threshold tuning via `--folder-similarity-threshold`

False-positive risk:

- medium; lower only when future content-hash folder comparison is added

Known limitation:

- This is not a full recursive content-hash equivalence engine.

## Engine 7: Utility and hygiene review modes

Status: implemented.

Modes:

- explicit RAW + JPEG pair mode
- empty file review mode
- empty folder review mode
- large-file review mode
- bad-extension detection mode

False-positive risk:

- low for empty files/folders and large-file review
- medium for bad-extension detection because detection covers a limited set of signatures

## Engine 8: Archive hygiene

Status: implemented for ZIP archives.

Modes:

- duplicate archive-member review mode
- empty archive review mode
- exact-mode `--scan-archives` support for ZIP members

False-positive risk:

- low to medium depending on the review mode

Known limitations:

- ZIP is the only archive format supported today.
- Archive member scanning currently reads each ZIP member into memory; resource-limit hardening remains planned.
- Archive pseudo-items are non-actionable by default.

## Planned engines / improvements

- broken-file review mode
- hard-link finder scan mode
- broader archive support such as 7z/rar/tar
- true rotation/flip-aware image grouping
- perceptual video fingerprints
- stronger acoustic fingerprints
- full recursive content-hash folder comparison
