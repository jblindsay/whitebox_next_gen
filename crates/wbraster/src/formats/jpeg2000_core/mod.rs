//! # GeoJP2
//!
//! Internal pure-Rust JPEG 2000 / GeoJP2 engine used by wbraster.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::let_and_return)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::needless_range_loop)]

pub mod boxes;
pub mod codestream;
pub mod entropy;
pub mod error;
pub mod geo_meta;
pub mod reader;
pub mod types;
pub mod wavelet;
pub mod writer;

pub use error::{Jp2Error, Result};
pub use geo_meta::CrsInfo;
pub use reader::GeoJp2;
pub use types::{BoundingBox, ColorSpace, CompressionMode, GeoTransform, PixelType, SampleFormat};
pub use writer::GeoJp2Writer;
