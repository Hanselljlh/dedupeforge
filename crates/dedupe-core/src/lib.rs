pub mod fs_walk;
pub mod hash;
pub mod scan;
pub mod verify;

pub use hash::HashAlgorithm;
pub use scan::{DuplicateGroup, DuplicateItem, ScanConfig, ScanReport, scan_exact};
