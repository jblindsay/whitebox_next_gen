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
        let signature_offset = find_hdf5_signature_offset(bytes)?;
        let version_offset = signature_offset + HDF5_SIGNATURE.len();
        if bytes.len() <= version_offset {
            return Err(WbhdfError::InvalidInput(
                "superblock parse requires at least 9 bytes".to_string(),
            ));
        }

        Ok(Self {
            version: bytes[version_offset],
        })
    }
}

pub fn validate_hdf5_signature(bytes: &[u8]) -> WbhdfResult<()> {
    find_hdf5_signature_offset(bytes).map(|_| ())
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

fn find_hdf5_signature_offset(bytes: &[u8]) -> WbhdfResult<usize> {
    if bytes.len() < HDF5_SIGNATURE.len() {
        return Err(WbhdfError::InvalidInput(
            "input is shorter than HDF5 signature".to_string(),
        ));
    }

    let search_len = usize::min(bytes.len(), 4096);
    if search_len < HDF5_SIGNATURE.len() {
        return Err(WbhdfError::InvalidInput(
            "input is shorter than HDF5 signature".to_string(),
        ));
    }

    for offset in 0..=search_len - HDF5_SIGNATURE.len() {
        if bytes[offset..offset + HDF5_SIGNATURE.len()] == HDF5_SIGNATURE {
            return Ok(offset);
        }
    }

    Err(WbhdfError::UnsupportedLayout(
        "missing HDF5 file signature".to_string(),
    ))
}

fn discover_top_level_groups_heuristic(bytes: &[u8]) -> Vec<String> {
    let candidates = [
        "GEDI04_B",
        "GEDI02_A",
        "BEAM0000",
        "gt1l",
        "gt1r",
        "gt2l",
        "gt2r",
        "gt3l",
        "gt3r",
        "HDFEOS",
        "MOD_Grid_500m_Surface_Reflectance",
        "VNP_Grid_1km_2D",
        "VIIRS_Swath_LSTE",
        "VIIRS-M3-SDR",
        "VIIRS-I4-IMG-EDR",
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
