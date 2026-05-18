pub mod fs_walk;
pub mod hash;
pub mod scan;
pub mod similar;
pub mod verify;

pub use fs_walk::{FileCollection, FileIdentity};
pub use hash::HashAlgorithm;
pub use scan::{
    scan, scan_exact, CacheConfig, DuplicateGroup, DuplicateItem, MatchRisk, ScanConfig, ScanMode,
    ScanReport,
};
