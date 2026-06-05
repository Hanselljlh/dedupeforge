# Feature matrix

This matrix describes the current `main` branch after a source review. “Implemented” means the code path exists and is covered by the current CLI and/or GUI; notes call out important limitations.

| Area | Current status | Notes |
|---|---|---|
| Exact duplicate scan | Implemented | Same size plus full hash; optional byte verification. |
| Partial hash prefilter | Implemented | Reduces unnecessary full reads before full hashing. |
| Fast hash choices | Implemented | BLAKE3 and XXH3-128. |
| Cryptographic hash | Implemented | SHA-256. |
| Byte-by-byte verification | Implemented | Optional final confirmation. |
| Protected/reference folders | Implemented | Protected files are preferred as keep items. |
| Human/JSON/CSV export | Implemented | Reports support all three formats. |
| SQLite cache | Implemented | Used by exact scans and image/video/audio fingerprint reuse. |
| Scan profiles and presets | Implemented | JSON profiles plus default/network/NAS presets. |
| Quarantine action | Implemented | Exact-mode action plans only. |
| Undo manifest / restore | Implemented | Quarantine batches write manifests and logs; restore reads manifest. |
| Saved action plans | Implemented | Plans can be saved, loaded, validated, and executed. |
| Hard-link replacement | Implemented | Advanced opt-in action; validates same-filesystem requirement. |
| Symlink replacement | Implemented | Advanced opt-in action; validates symlink support. |
| Filename similarity | Implemented | Token and edit-distance based, high false-positive risk. |
| Similar images | Implemented, first pass | Perceptual average hash plus EXIF reason support; true rotation/flip-aware grouping remains a known gap. |
| RAW + JPEG pairing | Implemented | Explicit mode and integrated image-pair reasoning by normalized basename. |
| Similar videos | Implemented, first pass | FFmpeg/ffprobe sampled-frame fingerprint plus duration threshold; not a robust perceptual video fingerprint. |
| Similar music/audio | Implemented, first pass | FFmpeg/ffprobe audio sample fingerprint plus duration and metadata reasons; not a robust acoustic fingerprint. |
| Duplicate folders | Implemented, first pass | File-tree overlap based on names/sizes and ignore patterns; not full recursive content-hash equivalence. |
| Empty files | Implemented | Low-risk review mode. |
| Empty folders | Implemented | Low-risk review mode. |
| Large files | Implemented | Low-risk review mode controlled by `--min-size`. |
| Bad extensions | Implemented | Medium-risk mode using a small magic-number detector. |
| Duplicate archive members | Implemented for ZIP | Reports duplicate members across `.zip` archives; archive pseudo-items are non-actionable. |
| Empty archives | Implemented for ZIP | Finds `.zip` archives with no file members. |
| Archive scanning in exact mode | Implemented for ZIP | `--scan-archives` adds duplicate ZIP-member reporting to exact scans. |
| Report database | Implemented | SQLite storage/list/load of scan reports for CLI and GUI workflows. |
| GUI | Implemented prototype | `egui`/`eframe` desktop app with scan setup, review, previews, actions, and report DB workflows. |
| CLI automation | Implemented, first pass | JSON/CSV reports, saved action plans, report DB storage, and non-interactive commands. |

## Comparison modes currently supported

### Exact file content

- same size
- same partial hash
- same full hash
- optional byte verification

### File properties / hygiene

- empty files
- empty folders
- large files above a threshold
- extension/content mismatch for common formats

### Similar names

- normalized filename
- token overlap
- Levenshtein-style edit distance
- ignored punctuation through normalization

### Similar images

- perceptual average hash
- configurable hash size
- Hamming distance threshold
- EXIF date in match reasons
- RAW + JPEG pair detection by normalized basename

Known limitation: `--image-rotation-invariant` is exposed, but scan grouping does not yet compare every generated rotation/flip variant.

### Similar video

- FFmpeg/ffprobe dependency detection
- duration tolerance
- sampled frame fingerprint comparison
- cache-backed fingerprints

Known limitation: fingerprints are cryptographic hashes of sampled output, so small media changes may avalanche rather than behave like a perceptual fingerprint.

### Similar music/audio

- FFmpeg/ffprobe dependency detection
- duration tolerance
- sampled audio fingerprint comparison
- optional metadata basis in reasons
- cache-backed fingerprints

Known limitation: this is not a mature acoustic fingerprinting engine.

### Duplicate folders

- file tree overlap
- filename and size signatures
- ignored file patterns

Known limitation: this is a first-pass folder similarity engine, not a full recursive content-hash folder equivalence engine.

### Archives

- ZIP archive member duplicate review
- ZIP empty archive review
- ZIP member scanning from exact mode

Known limitation: 7z/rar/tar and zip-bomb/resource-limit hardening are not implemented yet.

## Still planned / not implemented

- broken-file review mode
- hard-link finder scan mode
- broad archive formats beyond ZIP
- true rotation/flip-aware image grouping
- richer video/audio previews and mature perceptual media fingerprints
