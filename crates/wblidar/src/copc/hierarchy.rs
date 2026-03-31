//! COPC spatial hierarchy types.

use std::io::{Read, Write};
use crate::io::le;
use crate::Result;

/// Octree voxel key (level, x, y, z).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoxelKey {
    /// Octree depth level (0 = root).
    pub level: i32,
    /// Voxel X index at this level.
    pub x: i32,
    /// Voxel Y index at this level.
    pub y: i32,
    /// Voxel Z index at this level.
    pub z: i32,
}

impl VoxelKey {
    /// The root key.
    pub const ROOT: VoxelKey = VoxelKey { level: 0, x: 0, y: 0, z: 0 };

    /// Read from a little-endian stream.
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        Ok(VoxelKey {
            level: le::read_i32(r)?,
            x:     le::read_i32(r)?,
            y:     le::read_i32(r)?,
            z:     le::read_i32(r)?,
        })
    }

    /// Write to a little-endian stream.
    pub fn write<W: Write>(&self, w: &mut W) -> Result<()> {
        le::write_i32(w, self.level)?;
        le::write_i32(w, self.x)?;
        le::write_i32(w, self.y)?;
        le::write_i32(w, self.z)?;
        Ok(())
    }

    /// Return the 8 child keys of this voxel.
    pub fn children(self) -> [VoxelKey; 8] {
        let l = self.level + 1;
        let (x, y, z) = (self.x * 2, self.y * 2, self.z * 2);
        [
            VoxelKey { level: l, x,   y,   z   },
            VoxelKey { level: l, x:x+1, y,   z   },
            VoxelKey { level: l, x,   y:y+1, z   },
            VoxelKey { level: l, x:x+1, y:y+1, z   },
            VoxelKey { level: l, x,   y,   z:z+1 },
            VoxelKey { level: l, x:x+1, y,   z:z+1 },
            VoxelKey { level: l, x,   y:y+1, z:z+1 },
            VoxelKey { level: l, x:x+1, y:y+1, z:z+1 },
        ]
    }
}

/// A single entry in the COPC hierarchy page.
#[derive(Debug, Clone, Copy)]
pub struct CopcEntry {
    /// Voxel key.
    pub key: VoxelKey,
    /// Byte offset of the compressed chunk within the file.
    pub offset: u64,
    /// Byte size of the compressed chunk (-1 if node is a sub-page reference).
    pub byte_size: i32,
    /// Number of points in this chunk (-1 if sub-page reference).
    pub point_count: i32,
}

impl CopcEntry {
    /// Serialised size (32 bytes).
    pub const SIZE: usize = 32;

    /// Read a single entry.
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let key       = VoxelKey::read(r)?;
        let offset    = le::read_u64(r)?;
        let byte_size = le::read_i32(r)?;
        let point_count = le::read_i32(r)?;
        Ok(CopcEntry { key, offset, byte_size, point_count })
    }

    /// Write a single entry.
    pub fn write<W: Write>(&self, w: &mut W) -> Result<()> {
        self.key.write(w)?;
        le::write_u64(w, self.offset)?;
        le::write_i32(w, self.byte_size)?;
        le::write_i32(w, self.point_count)?;
        Ok(())
    }
}

/// The full COPC hierarchy page.
#[derive(Debug, Clone, Default)]
pub struct CopcHierarchy {
    /// All hierarchy entries.
    pub entries: Vec<CopcEntry>,
}

impl CopcHierarchy {
    /// Parse all entries from a raw byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut cur = std::io::Cursor::new(bytes);
        let count = bytes.len() / CopcEntry::SIZE;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(CopcEntry::read(&mut cur)?);
        }
        Ok(CopcHierarchy { entries })
    }

    /// Serialise all entries to bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(self.entries.len() * CopcEntry::SIZE);
        for e in &self.entries { e.write(&mut buf)?; }
        Ok(buf)
    }

    /// Look up an entry by voxel key.
    pub fn find(&self, key: VoxelKey) -> Option<&CopcEntry> {
        self.entries.iter().find(|e| e.key == key)
    }
}

/// COPC info block (160 bytes, stored in VLR record 1).
#[derive(Debug, Clone, Default)]
pub struct CopcInfo {
    /// Center X of root voxel.
    pub center_x: f64,
    /// Center Y of root voxel.
    pub center_y: f64,
    /// Center Z of root voxel.
    pub center_z: f64,
    /// Half-size (radius) of root voxel.
    pub halfsize: f64,
    /// Target spacing between points at the root level.
    pub spacing: f64,
    /// Byte offset to the hierarchy EVLR.
    pub hierarchy_root_offset: u64,
    /// Byte size of the hierarchy EVLR data.
    pub hierarchy_root_size: u64,
    /// GPS time minimum.
    pub gps_time_minimum: f64,
    /// GPS time maximum.
    pub gps_time_maximum: f64,
}

impl CopcInfo {
    /// Serialised size (160 bytes — reserved fields are zero-padded).
    pub const SIZE: usize = 160;

    /// Parse from a 160-byte slice.
    pub fn from_bytes(b: &[u8]) -> Result<Self> {
        let mut cur = std::io::Cursor::new(b);
        Ok(CopcInfo {
            center_x: le::read_f64(&mut cur)?,
            center_y: le::read_f64(&mut cur)?,
            center_z: le::read_f64(&mut cur)?,
            halfsize:  le::read_f64(&mut cur)?,
            spacing:   le::read_f64(&mut cur)?,
            hierarchy_root_offset: le::read_u64(&mut cur)?,
            hierarchy_root_size:   le::read_u64(&mut cur)?,
            gps_time_minimum: le::read_f64(&mut cur)?,
            gps_time_maximum: le::read_f64(&mut cur)?,
        })
    }

    /// Serialise to bytes (160 bytes, reserved fields zero-padded).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.extend_from_slice(&self.center_x.to_le_bytes());
        buf.extend_from_slice(&self.center_y.to_le_bytes());
        buf.extend_from_slice(&self.center_z.to_le_bytes());
        buf.extend_from_slice(&self.halfsize.to_le_bytes());
        buf.extend_from_slice(&self.spacing.to_le_bytes());
        buf.extend_from_slice(&self.hierarchy_root_offset.to_le_bytes());
        buf.extend_from_slice(&self.hierarchy_root_size.to_le_bytes());
        buf.extend_from_slice(&self.gps_time_minimum.to_le_bytes());
        buf.extend_from_slice(&self.gps_time_maximum.to_le_bytes());
        buf.resize(Self::SIZE, 0);
        buf
    }
}
