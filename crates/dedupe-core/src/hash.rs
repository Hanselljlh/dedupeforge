use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
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
    hash_file_limited(path, algorithm, None)
}

pub fn hash_file_prefix(path: &Path, algorithm: HashAlgorithm, max_bytes: u64) -> Result<String> {
    hash_file_limited(path, algorithm, Some(max_bytes))
}

fn hash_file_limited(path: &Path, algorithm: HashAlgorithm, limit: Option<u64>) -> Result<String> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut reader = BufReader::with_capacity(BUF_SIZE, file);
    let mut buf = vec![0u8; BUF_SIZE];
    let mut read_so_far = 0u64;

    match algorithm {
        HashAlgorithm::Blake3 => {
            let mut h = blake3::Hasher::new();
            loop {
                let wanted = wanted_len(limit, read_so_far, buf.len());
                if wanted == 0 { break; }
                let n = reader.read(&mut buf[..wanted])?;
                if n == 0 { break; }
                h.update(&buf[..n]);
                read_so_far += n as u64;
            }
            Ok(h.finalize().to_hex().to_string())
        }
        HashAlgorithm::Xxh3_128 => {
            let mut h = Xxh3::new();
            loop {
                let wanted = wanted_len(limit, read_so_far, buf.len());
                if wanted == 0 { break; }
                let n = reader.read(&mut buf[..wanted])?;
                if n == 0 { break; }
                h.update(&buf[..n]);
                read_so_far += n as u64;
            }
            Ok(format!("{:032x}", h.digest128()))
        }
        HashAlgorithm::Sha256 => {
            let mut h = Sha256::new();
            loop {
                let wanted = wanted_len(limit, read_so_far, buf.len());
                if wanted == 0 { break; }
                let n = reader.read(&mut buf[..wanted])?;
                if n == 0 { break; }
                h.update(&buf[..n]);
                read_so_far += n as u64;
            }
            Ok(hex::encode(h.finalize()))
        }
    }
}

fn wanted_len(limit: Option<u64>, read_so_far: u64, buf_len: usize) -> usize {
    match limit {
        None => buf_len,
        Some(limit) => limit.saturating_sub(read_so_far).min(buf_len as u64) as usize,
    }
}
