//! Cloud-Optimised Point Cloud (COPC) reader and writer.
//!
//! COPC is a LAS 1.4 file with a specific set of EVLRs that describe a
//! spatial hierarchy (EPT-style VoxelKey octree).  Points for each node in
//! the hierarchy are stored as LAZ-compressed chunks at known byte offsets.
//!
//! Reference: <https://copc.io/copc-specification-1.0.pdf>

pub mod hierarchy;
pub mod range_io;
pub mod reader;
pub mod writer;

pub use hierarchy::{CopcEntry, CopcHierarchy, VoxelKey};
pub use range_io::{ByteRangeSource, CachedRangeSource, LocalFileRangeSource};
#[cfg(feature = "copc-http")]
pub use range_io::HttpRangeSource;
pub use reader::{CopcHierarchyParseMode, CopcReader};
pub use writer::{CopcNodePointOrdering, CopcWriter, CopcWriterConfig};

/// COPC info VLR user ID.
pub const COPC_USER_ID: &str = "copc";
/// COPC info VLR record ID.
pub const COPC_INFO_RECORD_ID: u16 = 1;
/// COPC hierarchy EVLR record ID.
pub const COPC_HIERARCHY_RECORD_ID: u16 = 1000;
