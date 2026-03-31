use std::env;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};

use wblidar::las::reader::LasReader;
use wblidar::laz::laszip_chunk_table::{
    read_laszip_chunk_table_entries,
    read_laszip_chunk_table_header,
    read_laszip_chunk_table_pointer,
};
use wblidar::Result;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .expect("usage: inspect_standard_chunk_table <file.laz> [chunk_index]");
    let target_index = args.next().and_then(|v| v.parse::<usize>().ok());
    let file = File::open(&path)?;
    let mut las = LasReader::new(BufReader::new(file))?;
    let total_points = las.header().point_count();
    let data_start = las.offset_to_point_data();
    let file_len = las.inner_mut().seek(SeekFrom::End(0))?;

    let ptr = read_laszip_chunk_table_pointer(las.inner_mut(), data_start, file_len)?
        .expect("missing LASzip chunk-table pointer");
    let hdr = read_laszip_chunk_table_header(las.inner_mut(), ptr.chunk_table_offset, file_len)?;

    println!("file={}", path);
    println!("total_points={}", total_points);
    println!("data_start={}", data_start);
    println!("chunk_table_offset={}", ptr.chunk_table_offset);
    println!("chunk_count={}", hdr.chunk_count);

    for contains_point_count in [true, false] {
        las.inner_mut().seek(SeekFrom::Start(ptr.chunk_table_offset + 8))?;
        match read_laszip_chunk_table_entries(las.inner_mut(), hdr.chunk_count, contains_point_count) {
            Ok(entries) => {
                let total_bytes: u64 = entries.iter().map(|e| e.byte_count).sum();
                let total_entry_points: u64 = entries.iter().map(|e| e.point_count).sum();
                println!(
                    "contains_point_count={} entries={} total_bytes={} total_entry_points={}",
                    contains_point_count,
                    entries.len(),
                    total_bytes,
                    total_entry_points,
                );
                for (idx, entry) in entries.iter().take(5).enumerate() {
                    println!(
                        "  entry[{idx}] point_count={} byte_count={}",
                        entry.point_count,
                        entry.byte_count,
                    );
                }

                if contains_point_count {
                    if let Some(idx) = target_index {
                        if idx < entries.len() {
                            let mut cumulative = data_start + 8;
                            for e in entries.iter().take(idx) {
                                cumulative += e.byte_count;
                            }
                            let e = entries[idx];
                            println!(
                                "  target_entry[{idx}] start_offset={} point_count={} byte_count={}",
                                cumulative,
                                e.point_count,
                                e.byte_count,
                            );
                        } else {
                            println!(
                                "  target_entry[{idx}] out_of_range (entries={})",
                                entries.len()
                            );
                        }
                    }
                }
            }
            Err(err) => {
                println!("contains_point_count={} error={err}", contains_point_count);
            }
        }
    }

    Ok(())
}