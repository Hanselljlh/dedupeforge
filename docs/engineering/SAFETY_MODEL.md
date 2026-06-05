# Safety model

DedupeForge is review-first. Scans are read-only; cleanup requires explicit action plans and validation.

## Current scan safety

- Scanning never modifies files.
- Exact duplicate groups require same size and same full hash.
- Optional byte verification can confirm exact matches byte-for-byte.
- Protected/reference paths are marked during collection.
- Protected files are preferred as keep candidates.
- Every group gets exactly one suggested keep item.
- Zero-byte files are excluded from exact scans by default through `--min-size 1`.
- Similarity and hygiene modes include report-level risk labels.

## Current action safety

Actions are implemented, but only through explicit exact-mode action plans.

Implemented action behavior:

- dry-run action plan generation
- saved and loaded action plans
- validation of loaded plans against the current filesystem
- quarantine move
- hard-link replacement, opt-in
- symlink replacement, opt-in
- undo manifest writing
- action log writing
- restore from manifest
- execution-time hash revalidation before moving/replacing files

Hard safety rules:

- action plans are supported only for exact-mode scan reports
- protected items are skipped by the planner
- the planner rejects selecting every file in a duplicate group
- execution revalidates file availability, size, and full hash
- advanced link actions quarantine the original duplicate first so restore remains possible
- hard-link replacement validates same-filesystem requirements
- symlink replacement validates symlink support

## Suggested keep behavior

Scan default:

- protected file first, if present
- otherwise the first item after stable sorting by modified time, path depth, and path text

Action planner rules:

- `keep-suggested`
- `keep-newest`
- `keep-oldest`

GUI behavior:

- exact-mode reports can override the suggested keep item before building an action plan

## Non-exact results

Similarity and hygiene modes are review workflows. Current action planning rejects non-exact reports, including:

- similar names
- similar images
- RAW + JPEG pairs
- similar video/audio
- duplicate folders
- empty files/folders
- large files
- bad extensions
- archive hygiene modes

This keeps fuzzy and hygiene results from becoming destructive automatically.

## Archive safety

- ZIP archive member results are pseudo-items.
- Archive member pseudo-items are treated as protected/non-actionable review entries.
- ZIP support is current; broader archive formats and resource-limit hardening remain planned.

## Known safety gaps to improve

These are known code-review findings and should be addressed before broadening destructive workflows:

- quarantine destination names are lossy sanitized paths and can collide
- restore for link actions should verify the existing replacement is the link DedupeForge created before removing it
- archive member scanning should add resource limits for huge files and zip bombs
- FFmpeg/ffprobe calls should have timeouts
- non-exact modes should stay review-only until their false-positive behavior is better constrained
