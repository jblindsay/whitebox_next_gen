//! Integer delta/predictor codec used inside each LAZ chunk.
//!
//! LAZ applies a simple delta predictor per field. Adjacent points tend to be
//! geographically close, so field deltas are very small integers that arithmetic
//! coding (used by LASzip v2/v3) can represent very efficiently.

use std::io::{Cursor, Read};
use wide::f64x4;
use crate::point::{GpsTime, PointRecord, Rgb16};
use crate::io::le;

/// Encode a slice of points into a raw byte buffer using the delta predictor scheme.
/// (Internal use only; not called by current code.)
pub fn encode_chunk(points: &[PointRecord], has_gps: bool, has_rgb: bool) -> Vec<u8> {
    // Estimate buffer size: 30 bytes per point is a conservative lower bound.
    let mut buf = Vec::with_capacity(points.len() * 30);
    let mut prev_xi = 0i32;
    let mut prev_yi = 0i32;
    let mut prev_zi = 0i32;
    let mut prev_intensity = 0u16;
    let mut prev_gps_bits = 0u64;
    let mut prev_r = 0u16;
    let mut prev_g = 0u16;
    let mut prev_b = 0u16;

    for p in points {
        // Round all three coordinates in one SIMD op, then compute deltas.
        let xyz    = f64x4::new([p.x, p.y, p.z, 0.0]);
        let rounded: [f64; 4] = xyz.round().into();
        let xi = rounded[0] as i32;
        let yi = rounded[1] as i32;
        let zi = rounded[2] as i32;

        let dx = xi.wrapping_sub(prev_xi);
        let dy = yi.wrapping_sub(prev_yi);
        let dz = zi.wrapping_sub(prev_zi);
        prev_xi = xi; prev_yi = yi; prev_zi = zi;

        le::write_i32(&mut buf, dx).unwrap();
        le::write_i32(&mut buf, dy).unwrap();
        le::write_i32(&mut buf, dz).unwrap();

        let di = p.intensity.wrapping_sub(prev_intensity) as i16;
        prev_intensity = p.intensity;
        le::write_i16(&mut buf, di).unwrap();

        buf.push(p.return_number);
        buf.push(p.number_of_returns);
        buf.push(p.classification);
        buf.push(p.user_data);
        buf.push(p.flags);
        le::write_i16(&mut buf, p.scan_angle).unwrap();
        le::write_u16(&mut buf, p.point_source_id).unwrap();

        if has_gps {
            let gps_bits = p.gps_time.map_or(0.0, |g| g.0).to_bits();
            let dgps = gps_bits.wrapping_sub(prev_gps_bits) as i64;
            prev_gps_bits = gps_bits;
            buf.extend_from_slice(&dgps.to_le_bytes());
        }

        if has_rgb {
            let c = p.color.unwrap_or_default();
            let dr = c.red.wrapping_sub(prev_r) as i16;
            let dg = c.green.wrapping_sub(prev_g) as i16;
            let db = c.blue.wrapping_sub(prev_b) as i16;
            prev_r = c.red; prev_g = c.green; prev_b = c.blue;
            le::write_i16(&mut buf, dr).unwrap();
            le::write_i16(&mut buf, dg).unwrap();
            le::write_i16(&mut buf, db).unwrap();
        }
    }
    buf
}

/// Decode a raw byte buffer into a `Vec<PointRecord>`.
/// (Internal use only; not called by current code.)
pub fn decode_chunk(
    raw: &[u8],
    count: usize,
    has_gps: bool,
    has_rgb: bool,
) -> crate::Result<Vec<PointRecord>> {
    let mut cur = Cursor::new(raw);
    let mut points = vec![PointRecord::default(); count];

    let mut prev_xi = 0i32;
    let mut prev_yi = 0i32;
    let mut prev_zi = 0i32;
    let mut prev_intensity = 0u16;
    let mut prev_gps_bits = 0u64;
    let mut prev_r = 0u16;
    let mut prev_g = 0u16;
    let mut prev_b = 0u16;

    for p in &mut points {
        let dx = le::read_i32(&mut cur)?;
        let dy = le::read_i32(&mut cur)?;
        let dz = le::read_i32(&mut cur)?;
        prev_xi = prev_xi.wrapping_add(dx);
        prev_yi = prev_yi.wrapping_add(dy);
        prev_zi = prev_zi.wrapping_add(dz);
        p.x = f64::from(prev_xi);
        p.y = f64::from(prev_yi);
        p.z = f64::from(prev_zi);

        let di = le::read_i16(&mut cur)?;
        prev_intensity = prev_intensity.wrapping_add(di as u16);
        p.intensity = prev_intensity;

        p.return_number     = le::read_u8(&mut cur)?;
        p.number_of_returns = le::read_u8(&mut cur)?;
        p.classification    = le::read_u8(&mut cur)?;
        p.user_data         = le::read_u8(&mut cur)?;
        p.flags             = le::read_u8(&mut cur)?;
        p.scan_angle        = le::read_i16(&mut cur)?;
        p.point_source_id   = le::read_u16(&mut cur)?;

        if has_gps {
            let mut b8 = [0u8; 8];
            cur.read_exact(&mut b8).map_err(crate::Error::Io)?;
            let dgps = i64::from_le_bytes(b8);
            prev_gps_bits = prev_gps_bits.wrapping_add(dgps as u64);
            p.gps_time = Some(GpsTime(f64::from_bits(prev_gps_bits)));
        }

        if has_rgb {
            let dr = le::read_i16(&mut cur)?;
            let dg = le::read_i16(&mut cur)?;
            let db = le::read_i16(&mut cur)?;
            prev_r = prev_r.wrapping_add(dr as u16);
            prev_g = prev_g.wrapping_add(dg as u16);
            prev_b = prev_b.wrapping_add(db as u16);
            p.color = Some(Rgb16 { red: prev_r, green: prev_g, blue: prev_b });
        }
    }

    Ok(points)
}

