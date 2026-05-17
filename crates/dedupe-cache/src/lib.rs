use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedHash {
    pub hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CacheFileIdentity {
    pub device_id: String,
    pub inode: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HashScope {
    Partial { bytes_hashed: u64 },
    Full,
}

impl HashScope {
    fn label(self) -> &'static str {
        match self {
            HashScope::Partial { .. } => "partial",
            HashScope::Full => "full",
        }
    }

    fn bytes_hashed(self) -> i64 {
        match self {
            HashScope::Partial { bytes_hashed } => bytes_hashed as i64,
            HashScope::Full => 0,
        }
    }
}

pub struct Cache {
    conn: Connection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CacheLookupPolicy {
    pub modified_time_tolerance_secs: i64,
}

impl Cache {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create cache directory {}", parent.display())
            })?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("failed to open cache database {}", path.display()))?;

        let cache = Self { conn };
        cache.init()?;
        Ok(cache)
    }

    pub fn lookup_hash(
        &self,
        path: &Path,
        identity: Option<&CacheFileIdentity>,
        size: u64,
        modified_unix: Option<i64>,
        algorithm: &str,
        scope: HashScope,
        policy: CacheLookupPolicy,
    ) -> Result<Option<CachedHash>> {
        let canonical_path = canonicalize_for_lookup(path);
        let mut stmt = self.conn.prepare(
            "SELECT h.hash, f.modified_unix
             FROM files f
             JOIN hashes h ON h.file_id = f.id
             WHERE (f.path = ?1 OR (?2 IS NOT NULL AND ?3 IS NOT NULL AND f.device_id = ?2 AND f.inode = ?3))
               AND f.size = ?4
               AND h.algorithm = ?5
               AND h.scope = ?6
               AND h.bytes_hashed = ?7",
        )?;

        let found = stmt
            .query_row(
                params![
                    canonical_path.to_string_lossy().to_string(),
                    identity.map(|id| id.device_id.as_str()),
                    identity.map(|id| id.inode.as_str()),
                    size as i64,
                    algorithm,
                    scope.label(),
                    scope.bytes_hashed(),
                ],
                |row| {
                    Ok((
                        CachedHash { hash: row.get(0)? },
                        row.get::<_, Option<i64>>(1)?,
                    ))
                },
            )
            .optional()?;

        Ok(found.and_then(|(hash, cached_modified_unix)| {
            modified_time_matches(modified_unix, cached_modified_unix, policy).then_some(hash)
        }))
    }

    pub fn store_hash(
        &self,
        path: &Path,
        identity: Option<&CacheFileIdentity>,
        size: u64,
        modified_unix: Option<i64>,
        algorithm: &str,
        scope: HashScope,
        hash: &str,
    ) -> Result<()> {
        let canonical_path = canonicalize_for_lookup(path);
        let now = unix_now();
        self.conn.execute(
            "INSERT INTO files (path, canonical_path, device_id, inode, size, modified_unix, last_seen_unix)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(path) DO UPDATE SET
                canonical_path = excluded.canonical_path,
                device_id = excluded.device_id,
                inode = excluded.inode,
                size = excluded.size,
                modified_unix = excluded.modified_unix,
                last_seen_unix = excluded.last_seen_unix",
            params![
                canonical_path.to_string_lossy().to_string(),
                canonical_path.to_string_lossy().to_string(),
                identity.map(|id| id.device_id.as_str()),
                identity.map(|id| id.inode.as_str()),
                size as i64,
                modified_unix,
                now,
            ],
        )?;

        let file_id: i64 = self.conn.query_row(
            "SELECT id FROM files WHERE path = ?1",
            params![canonical_path.to_string_lossy().to_string()],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "INSERT INTO hashes (file_id, algorithm, scope, bytes_hashed, hash, created_unix)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(file_id, algorithm, scope, bytes_hashed) DO UPDATE SET
                hash = excluded.hash,
                created_unix = excluded.created_unix",
            params![
                file_id,
                algorithm,
                scope.label(),
                scope.bytes_hashed(),
                hash,
                now,
            ],
        )?;

        Ok(())
    }

    pub fn mark_seen(&self, path: &Path) -> Result<()> {
        let canonical_path = canonicalize_for_lookup(path);
        self.conn.execute(
            "UPDATE files SET last_seen_unix = ?2 WHERE path = ?1",
            params![canonical_path.to_string_lossy().to_string(), unix_now()],
        )?;
        Ok(())
    }

    fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                canonical_path TEXT,
                device_id TEXT,
                inode TEXT,
                size INTEGER NOT NULL,
                modified_unix INTEGER,
                created_unix INTEGER,
                last_seen_unix INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS hashes (
                file_id INTEGER NOT NULL,
                algorithm TEXT NOT NULL,
                scope TEXT NOT NULL,
                bytes_hashed INTEGER NOT NULL,
                hash TEXT NOT NULL,
                created_unix INTEGER NOT NULL,
                PRIMARY KEY (file_id, algorithm, scope, bytes_hashed),
                FOREIGN KEY (file_id) REFERENCES files(id)
             );
             CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);
             CREATE INDEX IF NOT EXISTS idx_files_identity ON files(device_id, inode);
             CREATE INDEX IF NOT EXISTS idx_hashes_lookup
                ON hashes(file_id, algorithm, scope, bytes_hashed);",
        )?;
        Ok(())
    }
}

fn canonicalize_for_lookup(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn modified_time_matches(
    current: Option<i64>,
    cached: Option<i64>,
    policy: CacheLookupPolicy,
) -> bool {
    match (current, cached) {
        (Some(current), Some(cached)) => {
            let delta = current.saturating_sub(cached).abs();
            delta <= policy.modified_time_tolerance_secs.max(0)
        }
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(name: &str) -> PathBuf {
        let unique = unix_now();
        std::env::temp_dir().join(format!("dedupeforge-cache-{unique}-{name}.sqlite3"))
    }

    #[test]
    fn stores_and_reuses_matching_hash() {
        let db = temp_db_path("reuse");
        let file = std::env::temp_dir().join("dedupeforge-cache-test.bin");
        fs::write(&file, b"abc").unwrap();

        let cache = Cache::open(&db).unwrap();
        cache
            .store_hash(
                &file,
                None,
                3,
                Some(10),
                "blake3",
                HashScope::Full,
                "hash123",
            )
            .unwrap();

        let found = cache
            .lookup_hash(
                &file,
                None,
                3,
                Some(10),
                "blake3",
                HashScope::Full,
                CacheLookupPolicy {
                    modified_time_tolerance_secs: 0,
                },
            )
            .unwrap();

        assert_eq!(found.unwrap().hash, "hash123");

        let _ = fs::remove_file(file);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn rejects_stale_hash_when_metadata_changes() {
        let db = temp_db_path("stale");
        let file = std::env::temp_dir().join("dedupeforge-cache-stale.bin");
        fs::write(&file, b"abc").unwrap();

        let cache = Cache::open(&db).unwrap();
        cache
            .store_hash(
                &file,
                None,
                3,
                Some(10),
                "sha256",
                HashScope::Partial { bytes_hashed: 2 },
                "hash456",
            )
            .unwrap();

        let found = cache
            .lookup_hash(
                &file,
                None,
                4,
                Some(10),
                "sha256",
                HashScope::Partial { bytes_hashed: 2 },
                CacheLookupPolicy {
                    modified_time_tolerance_secs: 0,
                },
            )
            .unwrap();

        assert!(found.is_none());

        let _ = fs::remove_file(file);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn accepts_small_modified_time_drift_with_tolerance() {
        let db = temp_db_path("mtime-tolerance");
        let file = std::env::temp_dir().join("dedupeforge-cache-tolerance.bin");
        fs::write(&file, b"abc").unwrap();

        let cache = Cache::open(&db).unwrap();
        cache
            .store_hash(
                &file,
                None,
                3,
                Some(10),
                "blake3",
                HashScope::Full,
                "hash789",
            )
            .unwrap();

        let found = cache
            .lookup_hash(
                &file,
                None,
                3,
                Some(12),
                "blake3",
                HashScope::Full,
                CacheLookupPolicy {
                    modified_time_tolerance_secs: 2,
                },
            )
            .unwrap();

        assert_eq!(found.unwrap().hash, "hash789");

        let _ = fs::remove_file(file);
        let _ = fs::remove_file(db);
    }

    #[test]
    fn reuses_hash_for_renamed_file_when_identity_matches() {
        let db = temp_db_path("identity-reuse");
        let original = std::env::temp_dir().join("dedupeforge-cache-original.bin");
        let renamed = std::env::temp_dir().join("dedupeforge-cache-renamed.bin");
        fs::write(&original, b"abc").unwrap();

        let identity = CacheFileIdentity {
            device_id: "device-1".to_string(),
            inode: "inode-42".to_string(),
        };

        let cache = Cache::open(&db).unwrap();
        cache
            .store_hash(
                &original,
                Some(&identity),
                3,
                Some(10),
                "blake3",
                HashScope::Full,
                "hash-identity",
            )
            .unwrap();

        let found = cache
            .lookup_hash(
                &renamed,
                Some(&identity),
                3,
                Some(10),
                "blake3",
                HashScope::Full,
                CacheLookupPolicy {
                    modified_time_tolerance_secs: 0,
                },
            )
            .unwrap();

        assert_eq!(found.unwrap().hash, "hash-identity");

        let _ = fs::remove_file(original);
        let _ = fs::remove_file(db);
    }
}
