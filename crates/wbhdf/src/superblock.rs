use crate::error::{WbhdfError, WbhdfResult};
use std::fs;
use std::path::Path;

pub const HDF5_SIGNATURE: [u8; 8] = [0x89, b'H', b'D', b'F', 0x0d, 0x0a, 0x1a, 0x0a];

/// Parsed superblock metadata required for container traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Superblock {
    pub version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerMetadata {
    pub superblock_version: u8,
    pub top_level_groups: Vec<String>,
}

impl Superblock {
    /// Parses a superblock from raw bytes.
    pub fn parse(bytes: &[u8]) -> WbhdfResult<Self> {
        if bytes.len() < 9 {
            return Err(WbhdfError::InvalidInput(
                "superblock parse requires at least 9 bytes".to_string(),
            ));
        }
        validate_hdf5_signature(bytes)?;

        Ok(Self { version: bytes[8] })
    }
}

pub fn validate_hdf5_signature(bytes: &[u8]) -> WbhdfResult<()> {
    if bytes.len() < HDF5_SIGNATURE.len() {
        return Err(WbhdfError::InvalidInput(
            "input is shorter than HDF5 signature".to_string(),
        ));
    }

    if bytes[..HDF5_SIGNATURE.len()] != HDF5_SIGNATURE {
        return Err(WbhdfError::UnsupportedLayout(
            "missing HDF5 file signature".to_string(),
        ));
    }

    Ok(())
}

/// Probes minimal metadata used by the Day 2 smoke-path target.
pub fn probe_file_metadata(path: &Path) -> WbhdfResult<ContainerMetadata> {
    let bytes = fs::read(path)?;
    let sb = Superblock::parse(&bytes)?;

    Ok(ContainerMetadata {
        superblock_version: sb.version,
        top_level_groups: discover_top_level_groups_heuristic(&bytes),
    })
}

fn discover_top_level_groups_heuristic(bytes: &[u8]) -> Vec<String> {
    let candidates = [
        "GEDI04_B",
        "gt1l",
        "gt1r",
        "gt2l",
        "gt2r",
        "gt3l",
        "gt3r",
        "HDFEOS",
        "MOD_Grid_500m_Surface_Reflectance",
        "VNP_Grid_1km_2D",
    ];

    let mut found = Vec::new();
    for c in candidates {
        if bytes.windows(c.len()).any(|w| w == c.as_bytes()) {
            found.push(c.to_string());
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{validate_hdf5_signature, HDF5_SIGNATURE};

    #[test]
    fn validates_hdf5_signature() {
        let mut buf = vec![0u8; 9];
        buf[..8].copy_from_slice(&HDF5_SIGNATURE);
        assert!(validate_hdf5_signature(&buf).is_ok());
    }

    #[test]
    fn rejects_bad_hdf5_signature() {
        let buf = vec![0u8; 9];
        assert!(validate_hdf5_signature(&buf).is_err());
    }
}
