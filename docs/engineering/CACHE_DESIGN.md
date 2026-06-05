# Cache design

The cache is implemented in `crates/dedupe-cache` as a SQLite database used by exact scans and media/image fingerprint reuse.

## Goals

- avoid rehashing unchanged files
- support partial and full hash reuse
- support image/video/audio fingerprint reuse
- tolerate small modified-time drift for network/NAS scans when configured
- reuse entries across renames when platform identity matches

## Current tables

The cache currently has two tables.

### `files`

| Column | Purpose |
|---|---|
| `id` | primary key |
| `path` | canonical lookup path, unique |
| `canonical_path` | canonical path copy for compatibility/reporting |
| `device_id` | optional platform device identifier |
| `inode` | optional platform inode/file identifier |
| `size` | file size in bytes |
| `modified_unix` | modified timestamp used for invalidation |
| `created_unix` | reserved/legacy timestamp field |
| `last_seen_unix` | last lookup/store time |

### `hashes`

| Column | Purpose |
|---|---|
| `file_id` | foreign key to `files` |
| `algorithm` | hash or fingerprint label |
| `scope` | `partial` or `full` |
| `bytes_hashed` | prefix length for partial hashes, `0` for full/fingerprint values |
| `hash` | hash/fingerprint hex string |
| `created_unix` | cache entry timestamp |

Primary key: `(file_id, algorithm, scope, bytes_hashed)`.

## Algorithm labels

Exact scans use labels from `HashAlgorithm`:

- `blake3`
- `xxh3_128`
- `sha256`

Media/image scans store fingerprints in the same `hashes` table with labels such as:

- `image-ahash-8`
- `image-ahash-16`
- `image-ahash-8-rot`
- `video-sampled-fingerprint`
- `audio-fingerprint`

## Lookup policy

A cache lookup matches by:

- exact cached canonical path, or matching `(device_id, inode)` when identity is available
- size
- algorithm/fingerprint label
- scope
- bytes hashed
- modified time within configured tolerance

If a matching entry is found, the cached hash/fingerprint is reused. If not, the caller computes and stores a new value.

## CLI cache support

The CLI accepts cache controls only for modes that currently use the cache:

- `exact`
- `similar-images`
- `similar-videos`
- `similar-audio`

Cache controls:

- `--cache`
- `--no-cache`
- `--cache-path <path>`
- `--clear-cache`
- `--rebuild-cache`
- `--cache-mtime-tolerance-secs <secs>`

Presets such as `network-tolerant` and `nas-conservative` enable cache behavior and mtime tolerance defaults.

## Current limitations

- no schema version table or migration system yet
- path and identity lookup can match older identity rows without an explicit ordering policy
- media/image fingerprints share the generic hash table rather than a richer media-specific schema
- `u64` sizes/byte counts are stored in SQLite integer fields
- ignore-pattern handling is not cache-specific and is not applied uniformly to every scan mode
