# Cache design

Status: implemented in first conservative form.

The cache makes repeated scans faster without becoming a source of false matches.

## Database

Use SQLite.

Reasons:

- local and portable
- no server required
- easy to inspect
- good enough for large local metadata caches

## Current tables

### files

```sql
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    canonical_path TEXT,
    device_id TEXT,
    inode TEXT,
    size INTEGER NOT NULL,
    modified_unix INTEGER,
    created_unix INTEGER,
    last_seen_unix INTEGER NOT NULL
);
```

### hashes

```sql
CREATE TABLE hashes (
    file_id INTEGER NOT NULL,
    algorithm TEXT NOT NULL,
    scope TEXT NOT NULL,
    bytes_hashed INTEGER,
    hash TEXT NOT NULL,
    created_unix INTEGER NOT NULL,
    PRIMARY KEY (file_id, algorithm, scope, bytes_hashed),
    FOREIGN KEY (file_id) REFERENCES files(id)
);
```

### media_hashes

```sql
CREATE TABLE media_hashes (
    file_id INTEGER NOT NULL,
    engine TEXT NOT NULL,
    algorithm TEXT NOT NULL,
    parameters_json TEXT NOT NULL,
    hash TEXT NOT NULL,
    created_unix INTEGER NOT NULL,
    PRIMARY KEY (file_id, engine, algorithm, parameters_json),
    FOREIGN KEY (file_id) REFERENCES files(id)
);
```

### scan_runs

```sql
CREATE TABLE scan_runs (
    id INTEGER PRIMARY KEY,
    started_unix INTEGER NOT NULL,
    finished_unix INTEGER,
    config_json TEXT NOT NULL,
    status TEXT NOT NULL
);
```

## Cache validity

A cached hash may be reused when:

- path matches
- or a stronger file identity matches where supported
- size matches
- modified timestamp matches, if available
- file has not been marked dirty

When uncertain, rehash.

## Network/NAS caution

Some network filesystems may provide unreliable inode/device data or timestamp precision.

The cache should support conservative mode:

- rely on path + size + modified time
- optionally allow small modified-time drift
- allow user to disable cache per scan

## Cache invalidation rule

Prefer false negatives over false positives.

It is acceptable to rehash unchanged files sometimes. It is not acceptable to trust stale hashes for changed files.
