# Feature matrix

This matrix describes the desired combined behavior. It is not a claim that all features are implemented.

| Area | MVP | Planned | Notes |
|---|---:|---:|---|
| Exact duplicate scan | Yes | Yes | same size plus full hash |
| Partial hash prefilter | Yes | Yes | reduces unnecessary full reads |
| Fast hash choices | Yes | Yes | BLAKE3 and XXH3 currently available |
| Cryptographic hash | Yes | Yes | SHA-256 currently available |
| Byte-by-byte verification | Yes | Yes | optional final confirmation |
| Protected/reference folders | Yes | Yes | protected files preferred as keep items |
| JSON/CSV export | Yes | Yes | useful for automation |
| SQLite cache | No | Yes | needed for large repeated scans |
| Quarantine action | No | Yes | first safe action to implement |
| Undo manifest | No | Yes | required before destructive workflows |
| Hard link replacement | No | Yes | local filesystem limitations apply |
| Symlink replacement | No | Yes | dangerous if misunderstood; should be advanced only |
| Filename similarity | No | Yes | token and string similarity methods |
| Similar images | No | Yes | perceptual hashes, rotation-aware options later |
| RAW + JPEG pairing | No | Yes | useful for photo workflows |
| Similar videos | No | Yes | FFmpeg frame hashing |
| Similar music | No | Yes | metadata plus audio fingerprinting |
| Duplicate folders | No | Yes | compare folder trees by policy |
| Archive scanning | No | Yes | zip/7z/rar later, likely optional |
| GUI | No | Yes | should call the same backend |
| CLI automation | Partial | Yes | future command set should support action manifests |

## Comparison modes to support eventually

### Exact file content

- same size
- same partial hash
- same full hash
- optional byte verification

### File properties

- name
- extension
- size
- created date
- modified date
- attributes
- hard link identity

### Similar names

- exact filename
- filename without extension
- normalized filename
- token/word overlap
- Levenshtein distance
- Ratcliff-Obershelp style similarity
- ignored punctuation/brackets/date patterns

### Similar images

- exact binary duplicate
- same pixels but different metadata
- perceptual hash match
- rotation/flip-aware match
- resized/recompressed match
- RAW + JPEG pair detection

### Similar video

- same duration
- near-same duration
- sampled frame perceptual hashes
- resolution/codec ignored
- optional audio track comparison

### Similar music/audio

- same file content
- same metadata tags
- similar tags
- same duration
- audio fingerprint match

### Duplicate folders

- same filenames
- same sizes
- same content hashes
- same tree after ignores
