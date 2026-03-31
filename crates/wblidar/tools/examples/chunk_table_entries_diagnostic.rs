use std::fs::File;
use std::io::{BufReader, Seek};

use wblidar::las::LasReader;
use wblidar::laz::{parse_laszip_vlr};
use wblidar::laz::laszip_chunk_table::{
    read_laszip_chunk_table_entries,
    read_laszip_chunk_table_header,
    read_laszip_chunk_table_pointer,
};

fn main() {
    let file_path = std::env::args()
        .nth(1)
        .expect("expected path to .las/.laz file");

    let file_len = std::fs::metadata(&file_path).expect("stat file").len();
    let file = File::open(&file_path).expect("open file");
    let mut las = LasReader::new(BufReader::new(file)).expect("read LAS header");
    let info = parse_laszip_vlr(las.vlrs()).expect("expected LASzip VLR");
    let data_start = las.offset_to_point_data();
    // Standard LASzip encodes per-chunk point counts only when chunk_size == u32::MAX
    // (variable-size chunks such as COPC).  Fixed-chunk streams store byte counts only.
    let contains_point_count = info.chunk_size == u32::MAX;

    println!("file={}", file_path);
    println!("file_len={}", file_len);
    println!("compressor={:?}", info.compressor);
    println!("data_start={}", data_start);
    println!("contains_point_count={}", contains_point_count);

    let inner = las.inner_mut();
    let ptr = read_laszip_chunk_table_pointer(inner, data_start, file_len)
        .expect("read pointer")
        .expect("chunk table pointer present");
    println!("chunk_table_offset={}", ptr.chunk_table_offset);

    let header = read_laszip_chunk_table_header(inner, ptr.chunk_table_offset, file_len)
        .expect("read table header");
    println!("chunk_table_version={}", header.version);
    println!("chunk_count={}", header.chunk_count);

    for mode in [contains_point_count, !contains_point_count] {
        inner
            .seek(std::io::SeekFrom::Start(ptr.chunk_table_offset + 8))
            .expect("seek to entries");
        println!("mode_contains_point_count={mode}");

        match read_laszip_chunk_table_entries(inner, header.chunk_count, mode) {
            Ok(entries) => {
                println!("decoded_entries={}", entries.len());
                let total_bytes: u64 = entries.iter().map(|e| e.byte_count).sum();
                let total_points: u64 = entries.iter().map(|e| e.point_count).sum();
                println!("sum_byte_count={}", total_bytes);
                println!("sum_point_count={}", total_points);
                println!("payload_available={}", ptr.chunk_table_offset.saturating_sub(data_start + 8));

                for (index, entry) in entries.iter().take(10).enumerate() {
                    println!(
                        "entry[{index}] point_count={} byte_count={}",
                        entry.point_count, entry.byte_count
                    );
                }

                let mut cumulative = data_start + 8;
                for (index, entry) in entries.iter().enumerate() {
                    cumulative = cumulative.saturating_add(entry.byte_count);
                    if cumulative > file_len {
                        println!(
                            "overflow_at_entry={} cumulative={} file_len={}",
                            index, cumulative, file_len
                        );
                        break;
                    }
                }
            }
            Err(err) => println!("decode_error={err:?}"),
        }
    }
}
