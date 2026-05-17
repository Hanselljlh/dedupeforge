# Scan pipeline

The exact duplicate pipeline is intentionally conservative.

```text
Input paths
  ↓
Recursive walk
  ↓
Metadata collection
  ↓
Filter by min size / hidden files
  ↓
Group by file size
  ↓
Discard unique sizes
  ↓
Partial hash likely candidates
  ↓
Discard unique partial hashes
  ↓
Full hash remaining candidates
  ↓
Optional byte-by-byte verification
  ↓
Create duplicate groups
  ↓
Choose suggested keep item
  ↓
Emit report
```

## Why group by size first

Two files cannot be exact duplicates if their sizes differ. Grouping by size avoids unnecessary hashing.

## Why partial hash first

Partial hashing reduces full-file reads. This is useful on large datasets and network storage.

A partial hash is not enough to declare duplicates. It only narrows candidates.

## Why full hash second

A full content hash is the main exact duplicate test.

Current choices:

- BLAKE3
- XXH3 128-bit
- SHA-256

## Why byte verification is optional

Byte-by-byte verification gives the strongest final confirmation but requires another full read of matching files. It is useful before destructive actions or when using non-cryptographic hashes.

## Candidate group reason strings

Every result group should include a reason.

Examples:

```text
same size + same full hash
same size + same full hash + byte-by-byte verified
similar image perceptual hash distance <= 8
same duration + matching sampled video frame hashes
```

## Future cache pipeline

With SQLite cache:

```text
Input paths
  ↓
Recursive walk
  ↓
File identity lookup
  ↓
Reuse valid cached hashes
  ↓
Hash only new/changed files
  ↓
Run grouping/matching
  ↓
Update cache
  ↓
Emit report
```

Cache entries should be invalidated when size, modified time, or file identity changes.
