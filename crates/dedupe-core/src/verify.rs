use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

const BUF_SIZE: usize = 1024 * 1024;

pub fn files_equal(a: &Path, b: &Path) -> Result<bool> {
    let file_a = File::open(a).with_context(|| format!("failed to open {}", a.display()))?;
    let file_b = File::open(b).with_context(|| format!("failed to open {}", b.display()))?;

    if file_a.metadata()?.len() != file_b.metadata()?.len() {
        return Ok(false);
    }

    let mut reader_a = BufReader::with_capacity(BUF_SIZE, file_a);
    let mut reader_b = BufReader::with_capacity(BUF_SIZE, file_b);
    let mut buf_a = vec![0u8; BUF_SIZE];
    let mut buf_b = vec![0u8; BUF_SIZE];

    loop {
        let n_a = reader_a.read(&mut buf_a)?;
        let n_b = reader_b.read(&mut buf_b)?;
        if n_a != n_b {
            return Ok(false);
        }
        if n_a == 0 {
            return Ok(true);
        }
        if buf_a[..n_a] != buf_b[..n_b] {
            return Ok(false);
        }
    }
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
        std::env::temp_dir().join(format!("dedupeforge-verify-{unique}-{name}"))
    }

    #[test]
    fn reports_equal_files() {
        let a = temp_file_path("a.bin");
        let b = temp_file_path("b.bin");
        fs::write(&a, b"same-bytes").unwrap();
        fs::write(&b, b"same-bytes").unwrap();

        assert!(files_equal(&a, &b).unwrap());

        fs::remove_file(a).unwrap();
        fs::remove_file(b).unwrap();
    }

    #[test]
    fn reports_different_files() {
        let a = temp_file_path("a.bin");
        let b = temp_file_path("b.bin");
        fs::write(&a, b"same-size").unwrap();
        fs::write(&b, b"diff-size").unwrap();

        assert!(!files_equal(&a, &b).unwrap());

        fs::remove_file(a).unwrap();
        fs::remove_file(b).unwrap();
    }
}
