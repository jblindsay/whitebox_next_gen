//! `wbphotogrammetry` — photogrammetry primitives for drone SfM workflows.
//!
//! This is a private crate (Option C) used exclusively by `wbtools_pro`.
//! The API may be stabilised and open-sourced in a later release.
//!
//! # Module structure
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`error`] | Error type and result alias |
//! | [`camera`] | Camera model and intrinsic parameters |
//! | [`ingest`] | Image-set ingestion and frame metadata |
//! | [`features`] | Feature detection and matching |
//! | [`alignment`] | Camera alignment / bundle adjustment |
//! | [`dense`] | Dense surface model reconstruction |
//! | [`mosaic`] | Orthomosaic generation |
//! | [`qa`] | Quality-assurance report and status classification |

pub mod error;
pub mod camera;
pub mod ingest;
pub mod features;
pub mod alignment;
pub mod dense;
pub mod mosaic;
pub mod qa;

// Convenient top-level re-exports used by `wbtools_pro`.
pub use error::{PhotogrammetryError, Result};
pub use camera::{CameraIntrinsics, CameraModel};
pub use ingest::{FrameMetadata, GpsCoordinate, ImageFrame, IngestChecks, compute_ingest_checks, ingest_image_set};
pub use features::{
    MatchStats,
    PairCorrespondences,
    run_feature_matching,
    run_feature_matching_brief,
    run_feature_matching_orb,
};
pub use alignment::{AlignmentResult, AlignmentStats, CameraPose, run_camera_alignment};
pub use dense::{DenseResult, DsmStats, run_dense_surface, run_dense_surface_with_dtm};
pub use mosaic::{MosaicResult, SeamStats, run_orthomosaic, run_orthomosaic_with_confidence};
pub use qa::{
    ProfileThresholds, ProcessingProfile, QaReport, QaStatus,
    build_qa_report,
};
