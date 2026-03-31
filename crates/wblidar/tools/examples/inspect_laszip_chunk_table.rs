use std::env;
use std::fs::File;
use std::io::BufReader;

use wblidar::las::reader::LasReader;
use wblidar::laz::laszip_chunk_table::{
    read_laszip_chunk_table_entries, read_laszip_chunk_table_header,
    read_laszip_chunk_table_pointer,
};
use wblidar::laz::parse_laszip_vlr;
use wblidar::Result;

fn main() -> Result<()> {
    let path = env::args().nth(1).expect("usage: inspect_laszip_chunk_table <file.laz>");
    let file = File::open(&path)?;
    let mut las = LasReader::new(BufReader::new(file))?;
    let hdr = las.header().clone();
    let file_len = las.inner_mut().get_ref().metadata()?.len();

    println!("file: {path}");
    println!(
        "pdrf(base)={} record_len={} points={} otp={}",
        hdr.point_data_format as u8,
        hdr.point_data_record_length,
        hdr.point_count_64.unwrap_or(hdr.legacy_point_count as u64),
        hdr.offset_to_point_data
    );

    let info = parse_laszip_vlr(las.vlrs());
    if let Some(i) = &info {
        println!(
            "laszip: compressor={:?} coder={} chunk_size={} items={}",
            i.compressor,
            i.coder,
            i.chunk_size,
            i.items.len()
        );
    } else {
        println!("laszip: none");
    }

    let data_start = las.offset_to_point_data();
    let inner = las.inner_mut();
    let ptr = read_laszip_chunk_table_pointer(inner, data_start, file_len)?;
    let Some(ptr) = ptr else {
        println!("pointer: none");
        return Ok(());
    };
    println!("pointer: data_start={} chunk_table_offset={}", ptr.data_start, ptr.chunk_table_offset);

    let header = read_laszip_chunk_table_header(inner, ptr.chunk_table_offset, file_len)?;
    println!("table: version={} count={}", header.version, header.chunk_count);

    for &contains_point_count in &[true, false] {
        let decoded = (|| -> Result<()> {
            use std::io::{Seek, SeekFrom};
            inner.seek(SeekFrom::Start(ptr.chunk_table_offset + 8))?;
            let entries = read_laszip_chunk_table_entries(inner, header.chunk_count, contains_point_count)?;
            let first: Vec<(u64, u64)> = entries.iter().take(5).map(|e| (e.point_count, e.byte_count)).collect();
            let sum_bytes: u128 = entries.iter().map(|e| e.byte_count as u128).sum();
            println!(
                "decode contains_point_count={contains_point_count}: ok entries={} sum_byte_count={} first5={:?}",
                entries.len(),
                sum_bytes,
                first,
            );
            Ok(())
        })();

        if let Err(e) = decoded {
            println!("decode contains_point_count={contains_point_count}: ERR {e}");
        }
    }

    Ok(())
}
