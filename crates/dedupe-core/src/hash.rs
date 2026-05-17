use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, BufReader};
use std::path::Path;
use xxhash_rust::xxh3::Xxh3;

const BUF_SIZE: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

fn hash_reader<R: Read>(reader: &mut R, algorithm: HashAlgorithm, max_bytes: Option<u64>) -> Result<String> {
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
        if wanted == 0 { break; }
        let n = reader.read(&mut buf[..wanted])?;
        if n == 0 { break; }
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
        if wanted == 0 { break; }
        let n = reader.read(&mut buf[..wanted])?;
        if n == 0 { break; }
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
        if wanted == 0 { break; }
        let n = reader.read(&mut buf[..wanted])?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
        read_so_far += n as u64;
    }

    Ok(hex::encode(hasher.finalize()))
}


#[cfg(test)]
mod tests {
        use super::*;
        use std::io::Write;
        use std::path::Path;
        use tempfile::NamedTempFile;

    fn temp_with(content: &[u8]) -> NamedTempFile {
                let mut f = NamedTempFile::new().unwrap();
                f.write_all(content).unwrap();
                f
    }

    #[test]
        fn blake3_same_content_same_hash() {
                    let a = temp_with(b"hello world");
                    let b = temp_with(b"hello world");
                    assert_eq!(
                                    hash_file(a.path(), HashAlgorithm::Blake3).unwrap(),
                                    hash_file(b.path(), HashAlgorithm::Blake3).unwrap(),
                                );
        }

    #[test]
        fn xxh3_same_content_same_hash() {
                    let a = temp_with(b"hello world");
                    let b = temp_with(b"hello world");
                    assert_eq!(
                                    hash_file(a.path(), HashAlgorithm::Xxh3_128).unwrap(),
                                    hash_file(b.path(), HashAlgorithm::Xxh3_128).unwrap(),
                                );
        }

    #[test]
        fn sha256_same_content_same_hash() {
                    let a = temp_with(b"hello world");
                    let b = temp_with(b"hello world");
                    assert_eq!(
                                    hash_file(a.path(), HashAlgorithm::Sha256).unwrap(),
                                    hash_file(b.path(), HashAlgorithm::Sha256).unwrap(),
                                );
        }

    #[test]
        fn different_content_produces_different_hash() {
                    let a = temp_with(b"hello world");
                    let b = temp_with(b"hello WORLD");
                    assert_ne!(
                                    hash_file(a.path(), HashAlgorithm::Blake3).unwrap(),
                                    hash_file(b.path(), HashAlgorithm::Blake3).unwrap(),
                                );
        }

    #[test]
        fn empty_file_hashes_consistently() {
                    let a = temp_with(b"");
                    let b = temp_with(b"");
                    assert_eq!(
                                    hash_file(a.path(), HashAlgorithm::Blake3).unwrap(),
                                    hash_file(b.path(), HashAlgorithm::Blake3).unwrap(),
                                );
        }

    #[test]
        fn prefix_hash_matches_full_hash_of_same_bytes() {
                    let full = temp_with(b"hello world");
                    let prefix_only = temp_with(b"hello");
                    let prefix_hash = hash_file_prefix(full.path(), HashAlgorithm::Blake3, 5).unwrap();
                    let full_small = hash_file(prefix_only.path(), HashAlgorithm::Blake3).unwrap();
                    assert_eq!(prefix_hash, full_small);
        }

    #[test]
        fn prefix_hash_differs_from_full_hash_when_file_is_longer() {
                    let f = temp_with(b"hello world");
                    let prefix = hash_file_prefix(f.path(), HashAlgorithm::Blake3, 5).unwrap();
                    let full = hash_file(f.path(), HashAlgorithm::Blake3).unwrap();
                    assert_ne!(prefix, full);
        }

    #[test]
        fn prefix_larger_than_file_equals_full_hash() {
                    let f = temp_with(b"tiny");
                    let prefix = hash_file_prefix(f.path(), HashAlgorithm::Blake3, 100_000).unwrap();
                    let full = hash_file(f.path(), HashAlgorithm::Blake3).unwrap();
                    assert_eq!(prefix, full);
        }

    #[test]
        fn algorithm_labels_are_correct() {
                    assert_eq!(HashAlgorithm::Blake3.label(), "blake3");
                    assert_eq!(HashAlgorithm::Xxh3_128.label(), "xxh3_128");
                    assert_eq!(HashAlgorithm::Sha256.label(), "sha256");
        }

    #[test]
        fn missing_file_returns_error() {
                    let result = hash_file(Path::new("/nonexistent/__dedupeforge_test__.bin"), HashAlgorithm::Blake3);
                    assert!(result.is_err());
        }
                    assert!(result.is_err());
        }
}
