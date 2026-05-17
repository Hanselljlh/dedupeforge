# Roadmap

## Phase 0: Repository foundation

Goal: make the project GitHub-ready.

- README
- project brief
- architecture docs
- safety model
- issue templates
- CI workflow
- license files
- contribution guide

## Phase 1: MVP exact duplicate scanner

Goal: reliable dry-run exact duplicate reports.

Implemented or in progress:

- recursive scan
- same-size grouping
- partial hash prefilter
- BLAKE3 / XXH3 / SHA-256
- optional byte verification
- protected/reference folders
- human/JSON/CSV output

Exit criteria:

- unit tests for hash and grouping behavior
- integration tests with fixture files
- clear output contract
- no destructive actions

## Phase 2: Cache and scan profiles

Goal: make repeated large scans practical.

Planned:

- SQLite cache
- file identity table
- file metadata table
- full hash table
- partial hash table
- cache invalidation on size/mtime/path changes
- scan profile config files

Exit criteria:

- repeated scans skip unchanged files
- cache can be rebuilt safely
- stale cache entries are not trusted

## Phase 3: Safe action system

Goal: allow cleanup without permanent damage.

Planned:

- action planner
- dry-run action output
- quarantine folder move
- undo manifest
- action log
- protected path enforcement
- group invariant: never remove all files in a group

Exit criteria:

- no action runs without an explicit action plan
- every action can be audited
- quarantine move can be reversed

## Phase 4: GUI prototype

Goal: AllDup-style review UI backed by the same engine.

Planned:

- source selection screen
- scan profile screen
- result table
- group details
- preview panel
- auto-select rules
- action queue

Exit criteria:

- GUI can run exact duplicate scan
- GUI can load/export scan results
- GUI does not implement separate scan logic

## Phase 5: Similar filename and folder engines

Goal: handle non-content workflows.

Planned:

- normalized filename matching
- token matching
- edit-distance matching
- folder tree comparison
- ignored file patterns

Exit criteria:

- match reasons are explainable
- threshold tuning is available
- false-positive risk is clearly labeled

## Phase 6: Similar images

Goal: photo cleanup workflows.

Planned:

- perceptual hashing
- selectable hash sizes
- Hamming distance threshold
- EXIF date support
- RAW + JPEG pair detection
- optional rotation/flip-aware slower mode

Exit criteria:

- exact image vs similar image are separate modes
- cache stores image hashes
- UI warns about false-positive risk

## Phase 7: Video and audio

Goal: large media library cleanup.

Planned:

- FFmpeg/ffprobe integration
- sampled video frame hashes
- duration tolerance
- audio fingerprinting
- music metadata comparison

Exit criteria:

- external dependency detection is clear
- long-running scans are resumable or cacheable
- match reasons show duration/hash/fingerprint basis

## Phase 8: Advanced cleanup features

Goal: power-user cleanup options.

Planned:

- hard link replacement
- symlink replacement
- archive scanning
- NAS/network conservative mode
- scheduled scan reports
- result database browser

Exit criteria:

- advanced features are disabled by default
- filesystem limitations are detected before action
- every advanced action is logged
