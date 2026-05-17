use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use xxhash_rust::xxh3::Xxh3;

const BUF_SIZE: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HashAlgorithm {
    Blake3,
    Xxh3_128,
    Sha256,
}

impl HashAlgorithm {
    pub fn label(self) -> &'static str {
        match self {
            HashAlgorithm::Blake3 => "blake3",
            HashAlgorithm::Xxh3_128 => "xxh3_128",
            HashAlgorithm::Sha256 => "sha256",
        }
    }
}

pub fn hash_file(path: &Path, algorithm: HashAlgorithm) -> Result<String> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut reader = BufReader::with_capacity(BUF_SIZE, file);
    hash_reader(&mut reader, algorithm, None)
}

pub fn hash_file_prefix(path: &Path, algorithm: HashAlgorithm, max_bytes: u64) -> Result<String> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut reader = BufReader::with_capacity(BUF_SIZE, file);
    hash_reader(&mut reader, algorithm, Some(max_bytes))
}

pub fn hash_bytes(bytes: &[u8], algorithm: HashAlgorithm) -> Result<String> {
    let mut cursor = Cursor::new(bytes);
    hash_reader(&mut cursor, algorithm, None)
}

fn hash_reader<R: Read>(
    reader: &mut R,
    algorithm: HashAlgorithm,
    max_bytes: Option<u64>,
) -> Result<String> {
    match algorithm {
        HashAlgorithm::Blake3 => hash_blake3(reader, max_bytes),
        HashAlgorithm::Xxh3_128 => hash_xxh3_128(reader, max_bytes),
        HashAlgorithm::Sha256 => hash_sha256(reader, max_bytes),
    }
}

fn next_read_len(max_bytes: Option<u64>, read_so_far: u64, buf_len: usize) -> usize {
    match max_bytes {
        None => buf_len,
        Some(limit) => {
            let remaining = limit.saturating_sub(read_so_far);
            remaining.min(buf_len as u64) as usize
        }
    }
}

fn hash_blake3<R: Read>(reader: &mut R, max_bytes: Option<u64>) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; BUF_SIZE];
    let mut read_so_far = 0u64;

    loop {
        let wanted = next_read_len(max_bytes, read_so_far, buf.len());
        if wanted == 0 {
            break;
        }
        let n = reader.read(&mut buf[..wanted])?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        read_so_far += n as u64;
    }

    Ok(hasher.finalize().to_hex().to_string())
}

fn hash_xxh3_128<R: Read>(reader: &mut R, max_bytes: Option<u64>) -> Result<String> {
    let mut hasher = Xxh3::new();
    let mut buf = vec![0u8; BUF_SIZE];
    let mut read_so_far = 0u64;

    loop {
        let wanted = next_read_len(max_bytes, read_so_far, buf.len());
        if wanted == 0 {
            break;
        }
        let n = reader.read(&mut buf[..wanted])?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        read_so_far += n as u64;
    }

    Ok(format!("{:032x}", hasher.digest128()))
}

fn hash_sha256<R: Read>(reader: &mut R, max_bytes: Option<u64>) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; BUF_SIZE];
    let mut read_so_far = 0u64;

    loop {
        let wanted = next_read_len(max_bytes, read_so_far, buf.len());
        if wanted == 0 {
            break;
        }
        let n = reader.read(&mut buf[..wanted])?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        read_so_far += n as u64;
    }

    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("dedupeforge-hash-{unique}-{name}"))
    }

    #[test]
    fn full_hash_is_stable_for_same_file() {
        let path = temp_file_path("stable.bin");
        fs::write(&path, b"duplicate-friendly-content").unwrap();

        let first = hash_file(&path, HashAlgorithm::Blake3).unwrap();
        let second = hash_file(&path, HashAlgorithm::Blake3).unwrap();

        assert_eq!(first, second);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn prefix_hash_changes_when_prefix_changes() {
        let a = temp_file_path("prefix-a.bin");
        let b = temp_file_path("prefix-b.bin");
        fs::write(&a, b"abcXXXXX").unwrap();
        fs::write(&b, b"xyzXXXXX").unwrap();

        let hash_a = hash_file_prefix(&a, HashAlgorithm::Sha256, 3).unwrap();
        let hash_b = hash_file_prefix(&b, HashAlgorithm::Sha256, 3).unwrap();

        assert_ne!(hash_a, hash_b);

        fs::remove_file(a).unwrap();
        fs::remove_file(b).unwrap();
    }

    #[test]
    fn prefix_hash_ignores_bytes_after_limit() {
        let a = temp_file_path("limit-a.bin");
        let b = temp_file_path("limit-b.bin");
        fs::write(&a, b"same-prefix-A").unwrap();
        fs::write(&b, b"same-prefix-B").unwrap();

        let hash_a = hash_file_prefix(&a, HashAlgorithm::Xxh3_128, 11).unwrap();
        let hash_b = hash_file_prefix(&b, HashAlgorithm::Xxh3_128, 11).unwrap();
        let full_a = hash_file(&a, HashAlgorithm::Xxh3_128).unwrap();
        let full_b = hash_file(&b, HashAlgorithm::Xxh3_128).unwrap();

        assert_eq!(hash_a, hash_b);
        assert_ne!(full_a, full_b);

        fs::remove_file(a).unwrap();
        fs::remove_file(b).unwrap();
    }
}
