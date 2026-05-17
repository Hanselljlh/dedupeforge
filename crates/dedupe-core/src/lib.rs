pub mod fs_walk;
pub mod hash;
pub mod scan;
pub mod verify;

pub use fs_walk::FileCollection;
pub use hash::HashAlgorithm;
pub use scan::{scan_exact, DuplicateGroup, DuplicateItem, ScanConfig, ScanReport};
