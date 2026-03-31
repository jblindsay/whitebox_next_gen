//! Minimal pure-Rust SQLite 3 engine sufficient for GeoPackage I/O.
//!
//! Implemented subset:
//! * Read: B-tree page scan, all serial types, overflow page follow
//! * Write: `CREATE TABLE`, `INSERT` (appending to rightmost leaf)
//! * No transactions, no indexes, no WHERE, no JOIN
//!
//! ## SQLite 3 file format summary
//! * Header: 100 bytes on page 1
//! * Page 1 byte 16-17: page size (u16 BE; value 1 means 65536)
//! * B-tree page types: 0x02=interior-index, 0x05=interior-table,
//!   0x0A=leaf-index, 0x0D=leaf-table
//! * Interior page header: 12 bytes (type + freeblock + ncells + content_start
//!   + fragmented + right_child)
//! * Leaf page header: 8 bytes (type + freeblock + ncells + content_start
//!   + fragmented)
//! * Cell pointer array: `ncells` × 2-byte BE offsets after the page header
//!
//! ## Record format
//! `[header_size varint][serial_type_0 varint]...[value_0 bytes]...`
//!
//! ## Serial type codes
//! 0=NULL, 1=i8, 2=i16, 3=i24, 4=i32, 5=i48, 6=i64, 7=f64,
//! 8=literal-0, 9=literal-1, ≥12 even=blob, ≥13 odd=text

use std::collections::HashMap;
use crate::error::{GeoError, Result};

// ══════════════════════════════════════════════════════════════════════════════
// SQLite value
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum SqlVal {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl SqlVal {
    pub fn as_i64(&self) -> Option<i64> {
        match self { Self::Int(v) => Some(*v), Self::Real(v) => Some(*v as i64), _ => None }
    }
    #[cfg(test)]
    pub fn as_f64(&self) -> Option<f64> {
        match self { Self::Real(v) => Some(*v), Self::Int(v) => Some(*v as f64), _ => None }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self { Self::Text(s) => Some(s.as_str()), _ => None }
    }
    pub fn as_blob(&self) -> Option<&[u8]> {
        match self { Self::Blob(b) => Some(b.as_slice()), _ => None }
    }
}

pub type Row = Vec<SqlVal>;

// ══════════════════════════════════════════════════════════════════════════════
// Varint
// ══════════════════════════════════════════════════════════════════════════════

/// Read a SQLite varint; return `(value, bytes_consumed)`.
fn read_varint(data: &[u8], mut pos: usize) -> (u64, usize) {
    let start = pos;
    let mut v = 0u64;
    for i in 0..9 {
        if pos >= data.len() { break; }
        let b = data[pos] as u64;
        pos += 1;
        if i == 8 {
            v = (v << 8) | b;
            return (v, pos - start);
        }
        v = (v << 7) | (b & 0x7F);
        if b & 0x80 == 0 {
            return (v, pos - start);
        }
    }
    (v, pos - start)
}

/// Encode a u64 as a SQLite varint.
fn write_varint(mut v: u64) -> Vec<u8> {
    if v <= 0x7F {
        return vec![v as u8];
    }

    // SQLite varint is big-endian base-128 (up to 9 bytes).
    // For values that fit in 56 bits, use 1..8 bytes.
    if v <= 0x00FF_FFFF_FFFF_FFFF {
        let mut tmp = [0u8; 8];
        let mut n = 0usize;
        while v > 0 {
            tmp[7 - n] = (v & 0x7F) as u8;
            v >>= 7;
            n += 1;
        }
        let mut out = tmp[(8 - n)..].to_vec();
        for i in 0..(out.len() - 1) {
            out[i] |= 0x80;
        }
        return out;
    }

    // 9-byte form: first 8 bytes carry 56 high bits in 7-bit chunks (all with
    // continuation bit), last byte carries low 8 bits.
    let mut out = vec![0u8; 9];
    out[8] = (v & 0xFF) as u8;
    v >>= 8;
    for i in (0..8).rev() {
        out[i] = ((v & 0x7F) as u8) | 0x80;
        v >>= 7;
    }
    out
}

// ══════════════════════════════════════════════════════════════════════════════
// Table metadata
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct TableMeta {
    pub root_page:  usize,   // 1-based page number
    pub columns:    Vec<String>,
}

// ══════════════════════════════════════════════════════════════════════════════
// Db
// ══════════════════════════════════════════════════════════════════════════════

/// An in-memory SQLite 3 database.
pub struct Db {
    pages:     Vec<Vec<u8>>,   // 0-indexed; page 1 = pages[0]
    page_size: usize,
    tables:    HashMap<String, TableMeta>,
}

impl Db {
    // ── Construction ──────────────────────────────────────────────────────────

    /// Load a SQLite database from raw bytes.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        if data.len() < 100 || &data[0..16] != b"SQLite format 3\0" {
            return Err(GeoError::NotGeoPackage("Not a SQLite 3 file".into()));
        }
        let ps_raw = u16::from_be_bytes([data[16], data[17]]) as usize;
        let page_size = if ps_raw == 1 { 65536 } else { ps_raw };
        if page_size < 512 || !page_size.is_power_of_two() {
            return Err(GeoError::NotGeoPackage(format!("invalid page size {page_size}")));
        }

        let mut pages = Vec::new();
        let mut off   = 0;
        while off + page_size <= data.len() {
            pages.push(data[off..off + page_size].to_vec());
            off += page_size;
        }

        let mut db = Self { pages, page_size, tables: HashMap::new() };
        db.load_schema()?;
        Ok(db)
    }

    /// Create a brand-new empty SQLite database.
    pub fn new_empty() -> Self {
        let ps = 65536usize;
        let mut p1 = vec![0u8; ps];

        // Header
        p1[0..16].copy_from_slice(b"SQLite format 3\0");
        // page size: value 1 encodes 65536 in SQLite header
        let ps_hdr: u16 = if ps == 65536 { 1 } else { ps as u16 };
        p1[16..18].copy_from_slice(&ps_hdr.to_be_bytes());
        p1[18] = 1; // file format write version
        p1[19] = 1; // file format read version
        p1[20] = 0; // reserved bytes per page
        p1[21] = 64; // max fraction
        p1[22] = 32; // min fraction
        p1[23] = 32; // leaf fraction
        p1[28..32].copy_from_slice(&1u32.to_be_bytes()); // page count
        p1[40..44].copy_from_slice(&1u32.to_be_bytes()); // schema cookie
        p1[44..48].copy_from_slice(&4u32.to_be_bytes()); // schema format 4
        p1[56..60].copy_from_slice(&1u32.to_be_bytes()); // text encoding UTF-8
        // GeoPackage application_id = 0x47503130 ("GP10")
        p1[68..72].copy_from_slice(&0x4750_3130u32.to_be_bytes());
        // sqlite_master is a leaf b-tree at page 1
        p1[100] = 0x0D; // leaf table
        p1[101..103].copy_from_slice(&0u16.to_be_bytes()); // freeblock = none
        p1[103..105].copy_from_slice(&0u16.to_be_bytes()); // ncells = 0
        p1[105..107].copy_from_slice(&(ps as u16).to_be_bytes()); // content area

        Self { pages: vec![p1], page_size: ps, tables: HashMap::new() }
    }

    /// Serialise to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pages.len() * self.page_size);
        for p in &self.pages { out.extend_from_slice(p); }
        out
    }

    // ── Schema ────────────────────────────────────────────────────────────────

    fn load_schema(&mut self) -> Result<()> {
        let rows = self.scan_btree(1, 100)?;
        for row in rows {
            if row.len() < 5 { continue; }
            let kind = row[0].as_str().unwrap_or("").to_ascii_lowercase();
            if kind != "table" { continue; }
            let name  = row[1].as_str().unwrap_or("").to_owned();
            let root  = row[3].as_i64().unwrap_or(0) as usize;
            let sql   = row[4].as_str().unwrap_or("").to_owned();
            let cols  = extract_column_names(&sql);
            self.tables.insert(name, TableMeta { root_page: root, columns: cols });
        }
        Ok(())
    }

    pub fn table_names(&self) -> Vec<&str> {
        self.tables.keys().map(|s| s.as_str()).collect()
    }

    pub fn table_meta(&self, name: &str) -> Option<&TableMeta> {
        self.tables.get(name)
    }

    // ── SELECT * ──────────────────────────────────────────────────────────────

    pub fn select_all(&self, table: &str) -> Result<Vec<Row>> {
        let root = self.tables.get(table)
            .map(|m| m.root_page)
            .ok_or_else(|| GeoError::GpkgSchema(format!("table '{table}' not found")))?;
        self.scan_btree(root, if root == 1 { 100 } else { 0 })
    }

    // ── B-tree scan ───────────────────────────────────────────────────────────

    fn scan_btree(&self, page_no: usize, header_offset: usize) -> Result<Vec<Row>> {
        let mut rows = Vec::new();
        self.walk_page(page_no, header_offset, &mut rows)?;
        Ok(rows)
    }

    fn walk_page(&self, page_no: usize, ho: usize, rows: &mut Vec<Row>) -> Result<()> {
        if page_no == 0 || page_no > self.pages.len() { return Ok(()); }
        let page = &self.pages[page_no - 1];
        if page.len() < ho + 8 { return Ok(()); }

        let page_type = page[ho];
        let n_cells   = u16::from_be_bytes([page[ho+3], page[ho+4]]) as usize;

        match page_type {
            0x0D => {
                // Leaf table page
                let cell_arr_start = ho + 8;
                for i in 0..n_cells {
                    let ptr_off = cell_arr_start + i * 2;
                    if ptr_off + 2 > page.len() { break; }
                    let cell_off = u16::from_be_bytes([page[ptr_off], page[ptr_off+1]]) as usize;
                    if let Some(row) = self.parse_leaf_cell(page, cell_off) {
                        rows.push(row);
                    }
                }
            }
            0x05 => {
                // Interior table page — recurse into children
                let cell_arr_start = ho + 12;
                let right_child = u32::from_be_bytes([page[ho+8], page[ho+9], page[ho+10], page[ho+11]]) as usize;
                for i in 0..n_cells {
                    let ptr_off = cell_arr_start + i * 2;
                    if ptr_off + 2 > page.len() { break; }
                    let cell_off = u16::from_be_bytes([page[ptr_off], page[ptr_off+1]]) as usize;
                    if cell_off + 4 > page.len() { continue; }
                    let child = u32::from_be_bytes([page[cell_off], page[cell_off+1], page[cell_off+2], page[cell_off+3]]) as usize;
                    self.walk_page(child, 0, rows)?;
                }
                self.walk_page(right_child, 0, rows)?;
            }
            _ => {} // leaf-index or interior-index: skip
        }
        Ok(())
    }

    fn parse_leaf_cell(&self, page: &[u8], off: usize) -> Option<Row> {
        if off >= page.len() { return None; }

        // [payload_size varint][rowid varint][payload]
        let (payload_size, n1) = read_varint(page, off);
        let (_rowid, n2) = read_varint(page, off + n1);
        let payload_size = usize::try_from(payload_size).ok()?;
        let payload_start = off + n1 + n2;

        if payload_start >= page.len() { return None; }

        let local_payload = Self::table_leaf_local_payload(payload_size, self.page_size);
        let local_end = payload_start.saturating_add(local_payload).min(page.len());
        let mut payload = Vec::with_capacity(payload_size);
        payload.extend_from_slice(&page[payload_start..local_end]);

        if payload_size > local_payload {
            let ptr_off = payload_start + local_payload;
            if ptr_off + 4 > page.len() {
                return None;
            }
            let first_overflow = u32::from_be_bytes([
                page[ptr_off],
                page[ptr_off + 1],
                page[ptr_off + 2],
                page[ptr_off + 3],
            ]) as usize;
            let remaining = payload_size - local_payload;
            let overflow = self.read_overflow_payload(first_overflow, remaining)?;
            payload.extend_from_slice(&overflow);
        }

        if payload.len() != payload_size {
            return None;
        }

        parse_record(&payload)
    }

    // ── INSERT ────────────────────────────────────────────────────────────────

    /// Append a row to a table and return the new rowid.
    pub fn insert(&mut self, table: &str, values: Vec<SqlVal>) -> Result<i64> {
        let root = self.tables.get(table)
            .map(|m| m.root_page)
            .ok_or_else(|| GeoError::GpkgSchema(format!("table '{table}' not found")))?;

        // Determine next rowid from current row count
        let existing = self.scan_btree(root, if root == 1 { 100 } else { 0 })?;
        let rowid    = (existing.len() as i64) + 1;

        let cell = self.build_leaf_cell_with_overflow(rowid as u64, &values)?;
        let leaf  = self.find_rightmost_leaf(root, if root == 1 { 100 } else { 0 });
        self.insert_cell(leaf, cell)?;
        Ok(rowid)
    }

    fn find_rightmost_leaf(&self, page_no: usize, ho: usize) -> usize {
        if page_no == 0 || page_no > self.pages.len() { return page_no; }
        let page = &self.pages[page_no - 1];
        if page.len() <= ho { return page_no; }
        if page[ho] == 0x05 {
            let right = u32::from_be_bytes([page[ho+8], page[ho+9], page[ho+10], page[ho+11]]) as usize;
            if right > 0 { return self.find_rightmost_leaf(right, 0); }
        }
        page_no
    }

    fn insert_cell(&mut self, page_no: usize, cell: Vec<u8>) -> Result<()> {
        if page_no == 0 || page_no > self.pages.len() {
            return Err(GeoError::Sqlite(format!("page {page_no} out of range")));
        }
        let ho = if page_no == 1 { 100 } else { 0 };
        let ps = self.page_size;

        let n_cells       = u16::from_be_bytes([self.pages[page_no-1][ho+3], self.pages[page_no-1][ho+4]]) as usize;
        let content_start_raw = u16::from_be_bytes([self.pages[page_no-1][ho+5], self.pages[page_no-1][ho+6]]) as usize;
        let content_start = if content_start_raw == 0 { ps } else { content_start_raw };

        let cell_arr_end  = ho + 8 + n_cells * 2;
        let free_space    = content_start.saturating_sub(cell_arr_end);

        if cell.len() + 2 > free_space {
            return self.spill_to_new_page(page_no, cell);
        }

        let new_content = content_start - cell.len();
        let p = &mut self.pages[page_no - 1];
        p[new_content..new_content + cell.len()].copy_from_slice(&cell);
        let ptr_off = ho + 8 + n_cells * 2;
        p[ptr_off..ptr_off+2].copy_from_slice(&(new_content as u16).to_be_bytes());
        let new_n = (n_cells + 1) as u16;
        p[ho+3..ho+5].copy_from_slice(&new_n.to_be_bytes());
        p[ho+5..ho+7].copy_from_slice(&(new_content as u16).to_be_bytes());
        Ok(())
    }

    fn max_rowid_in_leaf_page(&self, page_no: usize, ho: usize) -> Option<u64> {
        if page_no == 0 || page_no > self.pages.len() {
            return None;
        }
        let page = &self.pages[page_no - 1];
        if page.len() < ho + 8 || page[ho] != 0x0D {
            return None;
        }
        let n_cells = u16::from_be_bytes([page[ho + 3], page[ho + 4]]) as usize;
        let cell_arr_start = ho + 8;
        let mut max_rowid = None;
        for i in 0..n_cells {
            let ptr_off = cell_arr_start + i * 2;
            if ptr_off + 2 > page.len() {
                break;
            }
            let cell_off = u16::from_be_bytes([page[ptr_off], page[ptr_off + 1]]) as usize;
            if cell_off >= page.len() {
                continue;
            }
            let (_payload_size, n1) = read_varint(page, cell_off);
            let (rowid, _n2) = read_varint(page, cell_off + n1);
            max_rowid = Some(max_rowid.map_or(rowid, |m: u64| m.max(rowid)));
        }
        max_rowid
    }

    fn find_parent_of_page(&self, child_page_no: usize) -> Option<(usize, usize)> {
        for pno in 1..=self.pages.len() {
            let ho = if pno == 1 { 100 } else { 0 };
            let page = &self.pages[pno - 1];
            if page.len() < ho + 12 || page[ho] != 0x05 {
                continue;
            }
            let n_cells = u16::from_be_bytes([page[ho + 3], page[ho + 4]]) as usize;
            let right = u32::from_be_bytes([page[ho + 8], page[ho + 9], page[ho + 10], page[ho + 11]]) as usize;
            if right == child_page_no {
                return Some((pno, ho));
            }
            let cell_arr_start = ho + 12;
            for i in 0..n_cells {
                let ptr_off = cell_arr_start + i * 2;
                if ptr_off + 2 > page.len() {
                    break;
                }
                let cell_off = u16::from_be_bytes([page[ptr_off], page[ptr_off + 1]]) as usize;
                if cell_off + 4 > page.len() {
                    continue;
                }
                let child = u32::from_be_bytes([
                    page[cell_off],
                    page[cell_off + 1],
                    page[cell_off + 2],
                    page[cell_off + 3],
                ]) as usize;
                if child == child_page_no {
                    return Some((pno, ho));
                }
            }
        }
        None
    }

    fn append_right_split_to_parent(
        &mut self,
        parent_page_no: usize,
        parent_ho: usize,
        old_right_child: usize,
        separator_key: u64,
        new_right_child: usize,
    ) -> Result<()> {
        let n_cells = u16::from_be_bytes([
            self.pages[parent_page_no - 1][parent_ho + 3],
            self.pages[parent_page_no - 1][parent_ho + 4],
        ]) as usize;
        let content_start_raw = u16::from_be_bytes([
            self.pages[parent_page_no - 1][parent_ho + 5],
            self.pages[parent_page_no - 1][parent_ho + 6],
        ]) as usize;
        let content_start = if content_start_raw == 0 {
            self.page_size
        } else {
            content_start_raw
        };

        let current_right = u32::from_be_bytes([
            self.pages[parent_page_no - 1][parent_ho + 8],
            self.pages[parent_page_no - 1][parent_ho + 9],
            self.pages[parent_page_no - 1][parent_ho + 10],
            self.pages[parent_page_no - 1][parent_ho + 11],
        ]) as usize;
        if current_right != old_right_child {
            return Err(GeoError::Sqlite(
                "parent split-link expects split on rightmost child".to_owned(),
            ));
        }

        let mut cell = Vec::new();
        cell.extend_from_slice(&(old_right_child as u32).to_be_bytes());
        cell.extend_from_slice(&write_varint(separator_key));

        let cell_arr_end = parent_ho + 12 + n_cells * 2;
        let free_space = content_start.saturating_sub(cell_arr_end);
        if cell.len() + 2 > free_space {
            return Err(GeoError::Sqlite(
                "interior parent overflow during right split link".to_owned(),
            ));
        }

        let new_content = content_start - cell.len();
        let p = &mut self.pages[parent_page_no - 1];
        p[new_content..new_content + cell.len()].copy_from_slice(&cell);
        let ptr_off = parent_ho + 12 + n_cells * 2;
        p[ptr_off..ptr_off + 2].copy_from_slice(&(new_content as u16).to_be_bytes());
        let new_n = (n_cells + 1) as u16;
        p[parent_ho + 3..parent_ho + 5].copy_from_slice(&new_n.to_be_bytes());
        p[parent_ho + 5..parent_ho + 7].copy_from_slice(&(new_content as u16).to_be_bytes());
        p[parent_ho + 8..parent_ho + 12].copy_from_slice(&(new_right_child as u32).to_be_bytes());
        Ok(())
    }

    fn spill_to_new_page(&mut self, old_page_no: usize, cell: Vec<u8>) -> Result<()> {
        let ps        = self.page_size;
        let ho = if old_page_no == 1 { 100 } else { 0 };
        if old_page_no == 0 || old_page_no > self.pages.len() {
            return Err(GeoError::Sqlite(format!("page {old_page_no} out of range")));
        }
        if self.pages[old_page_no - 1][ho] != 0x0D {
            return Err(GeoError::Sqlite(format!(
                "spill expected leaf-table page, found type 0x{:02X}",
                self.pages[old_page_no - 1][ho]
            )));
        }

        let separator = self.max_rowid_in_leaf_page(old_page_no, ho)
            .ok_or_else(|| GeoError::Sqlite("cannot split empty leaf page".to_owned()))?;

        // Non-root leaf split: keep old page as leaf, allocate a new right leaf,
        // then link the split into the parent interior page.
        if let Some((parent_page_no, parent_ho)) = self.find_parent_of_page(old_page_no) {
            let new_right_page_no = self.pages.len() + 1;
            let mut right_leaf = vec![0u8; ps];
            right_leaf[0] = 0x0D;
            right_leaf[3..5].copy_from_slice(&0u16.to_be_bytes());
            right_leaf[5..7].copy_from_slice(&(ps as u16).to_be_bytes());
            self.pages.push(right_leaf);

            let pc = self.pages.len() as u32;
            self.pages[0][28..32].copy_from_slice(&pc.to_be_bytes());

            self.append_right_split_to_parent(
                parent_page_no,
                parent_ho,
                old_page_no,
                separator,
                new_right_page_no,
            )?;

            return self.insert_cell(new_right_page_no, cell);
        }

        // Copy the current full leaf into a new left child page.
        let left_page_no = self.pages.len() + 1;
        self.pages.push(self.pages[old_page_no - 1].clone());

        // Allocate a fresh right leaf page where the new row will be inserted.
        let right_page_no = self.pages.len() + 1;
        let mut right_leaf = vec![0u8; ps];
        right_leaf[0] = 0x0D;
        right_leaf[3..5].copy_from_slice(&0u16.to_be_bytes());
        right_leaf[5..7].copy_from_slice(&(ps as u16).to_be_bytes());
        self.pages.push(right_leaf);

        // Update page count in header
        let pc = self.pages.len() as u32;
        self.pages[0][28..32].copy_from_slice(&pc.to_be_bytes());

        // Convert old page into an interior-table page:
        // left child = copied full leaf, right child = new right leaf.
        let mut interior_cell = Vec::new();
        interior_cell.extend_from_slice(&(left_page_no as u32).to_be_bytes());
        interior_cell.extend_from_slice(&write_varint(separator));
        let content_start = ps.saturating_sub(interior_cell.len());
        if content_start <= ho + 12 {
            return Err(GeoError::Sqlite("interior split cell does not fit page".to_owned()));
        }

        let p = &mut self.pages[old_page_no - 1];
        for b in &mut p[ho..] {
            *b = 0;
        }
        p[ho] = 0x05; // interior table b-tree page
        p[ho + 3..ho + 5].copy_from_slice(&1u16.to_be_bytes()); // one cell
        p[ho + 5..ho + 7].copy_from_slice(&(content_start as u16).to_be_bytes());
        p[ho + 8..ho + 12].copy_from_slice(&(right_page_no as u32).to_be_bytes());
        p[ho + 12..ho + 14].copy_from_slice(&(content_start as u16).to_be_bytes());
        p[content_start..content_start + interior_cell.len()].copy_from_slice(&interior_cell);

        self.insert_cell(right_page_no, cell)
    }

    // ── CREATE TABLE ─────────────────────────────────────────────────────────

    pub fn create_table(&mut self, sql: &str) -> Result<()> {
        let name = extract_table_name(sql)
            .ok_or_else(|| GeoError::Sqlite(format!("cannot parse table name from: {sql}")))?;
        if self.tables.contains_key(&name) { return Ok(()); }

        // Allocate a new B-tree page for this table
        let ps = self.page_size;
        let new_page_no = self.pages.len() + 1;
        let mut new_p = vec![0u8; ps];
        new_p[0] = 0x0D;
        new_p[3..5].copy_from_slice(&0u16.to_be_bytes());
        new_p[5..7].copy_from_slice(&(ps as u16).to_be_bytes());
        self.pages.push(new_p);

        let pc = self.pages.len() as u32;
        self.pages[0][28..32].copy_from_slice(&pc.to_be_bytes());

        // Insert row into sqlite_master (page 1)
        let existing = self.scan_btree(1, 100)?;
        let rowid    = (existing.len() as i64) + 1;
        let master_row = vec![
            SqlVal::Text("table".into()),
            SqlVal::Text(name.clone()),
            SqlVal::Text(name.clone()),
            SqlVal::Int(new_page_no as i64),
            SqlVal::Text(sql.to_owned()),
        ];
        let cell = self.build_leaf_cell_with_overflow(rowid as u64, &master_row)?;
        self.insert_cell(1, cell)?;

        let cols = extract_column_names(sql);
        self.tables.insert(name, TableMeta { root_page: new_page_no, columns: cols });
        Ok(())
    }

    fn table_leaf_local_payload(payload_size: usize, page_size: usize) -> usize {
        let usable = page_size;
        let max_local = usable.saturating_sub(35);
        let min_local = (((usable.saturating_sub(12)) * 32) / 255).saturating_sub(23);
        if payload_size <= max_local {
            payload_size
        } else {
            let mut local = min_local + ((payload_size - min_local) % (usable.saturating_sub(4).max(1)));
            if local > max_local {
                local = min_local;
            }
            local
        }
    }

    fn read_overflow_payload(&self, first_page: usize, total_len: usize) -> Option<Vec<u8>> {
        if total_len == 0 {
            return Some(Vec::new());
        }
        let mut out = Vec::with_capacity(total_len);
        let mut remaining = total_len;
        let mut page_no = first_page;
        let mut hops = 0usize;
        while remaining > 0 {
            if page_no == 0 || page_no > self.pages.len() {
                return None;
            }
            if hops > self.pages.len() {
                return None;
            }
            hops += 1;

            let page = &self.pages[page_no - 1];
            if page.len() < 4 {
                return None;
            }
            let next = u32::from_be_bytes([page[0], page[1], page[2], page[3]]) as usize;
            let chunk = remaining.min(self.page_size.saturating_sub(4));
            if page.len() < 4 + chunk {
                return None;
            }
            out.extend_from_slice(&page[4..4 + chunk]);
            remaining -= chunk;
            page_no = next;
        }
        Some(out)
    }

    fn write_overflow_chain(&mut self, bytes: &[u8]) -> Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        let chunk_size = self.page_size.saturating_sub(4).max(1);
        let page_count = bytes.len().div_ceil(chunk_size);
        let first_page_no = self.pages.len() + 1;

        for page_idx in 0..page_count {
            let start = page_idx * chunk_size;
            let end = (start + chunk_size).min(bytes.len());
            let next_page_no = if page_idx + 1 < page_count {
                first_page_no + page_idx + 1
            } else {
                0
            };
            let mut page = vec![0u8; self.page_size];
            page[0..4].copy_from_slice(&(next_page_no as u32).to_be_bytes());
            page[4..4 + (end - start)].copy_from_slice(&bytes[start..end]);
            self.pages.push(page);
        }

        let pc = self.pages.len() as u32;
        self.pages[0][28..32].copy_from_slice(&pc.to_be_bytes());
        Ok(first_page_no)
    }

    fn build_leaf_cell_with_overflow(&mut self, rowid: u64, values: &[SqlVal]) -> Result<Vec<u8>> {
        let record = build_record_payload(values);
        let payload_size = record.len();
        let local_payload = Self::table_leaf_local_payload(payload_size, self.page_size);

        let mut cell = Vec::new();
        cell.extend_from_slice(&write_varint(payload_size as u64));
        cell.extend_from_slice(&write_varint(rowid));
        cell.extend_from_slice(&record[..local_payload]);

        if payload_size > local_payload {
            let first_overflow = self.write_overflow_chain(&record[local_payload..])?;
            cell.extend_from_slice(&(first_overflow as u32).to_be_bytes());
        }

        Ok(cell)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Record codec
// ══════════════════════════════════════════════════════════════════════════════

fn parse_record(data: &[u8]) -> Option<Row> {
    if data.is_empty() { return None; }
    let (hdr_size, hn) = read_varint(data, 0);
    let hdr_size = hdr_size as usize;
    if hdr_size > data.len() { return None; }

    let mut types = Vec::new();
    let mut pos   = hn;
    while pos < hdr_size {
        let (t, n) = read_varint(data, pos);
        types.push(t);
        pos += n;
    }

    let mut row  = Vec::with_capacity(types.len());
    let mut dpos = hdr_size;

    for &t in &types {
        let (val, sz) = decode_serial(data, dpos, t);
        row.push(val);
        dpos += sz;
    }
    Some(row)
}

fn decode_serial(data: &[u8], pos: usize, t: u64) -> (SqlVal, usize) {
    let get = |off: usize, n: usize| -> Vec<u8> {
        data.get(pos+off..pos+off+n).unwrap_or(&[]).to_vec()
    };
    match t {
        0 => (SqlVal::Null, 0),
        1 => {
            let v = data.get(pos).copied().unwrap_or(0) as i8 as i64;
            (SqlVal::Int(v), 1)
        }
        2 => {
            let b: [u8;2] = get(0,2).try_into().unwrap_or([0;2]);
            (SqlVal::Int(i16::from_be_bytes(b) as i64), 2)
        }
        3 => {
            let b = get(0,3);
            let v = (b.get(0).copied().unwrap_or(0) as i32) << 16
                  | (b.get(1).copied().unwrap_or(0) as i32) << 8
                  | (b.get(2).copied().unwrap_or(0) as i32);
            let v = if v & 0x80_0000 != 0 { v | !0xFF_FFFF } else { v };
            (SqlVal::Int(v as i64), 3)
        }
        4 => {
            let b: [u8;4] = get(0,4).try_into().unwrap_or([0;4]);
            (SqlVal::Int(i32::from_be_bytes(b) as i64), 4)
        }
        5 => {
            let b = get(0,6);
            let v: i64 = (b.get(0).copied().unwrap_or(0) as i64) << 40
                       | (b.get(1).copied().unwrap_or(0) as i64) << 32
                       | (b.get(2).copied().unwrap_or(0) as i64) << 24
                       | (b.get(3).copied().unwrap_or(0) as i64) << 16
                       | (b.get(4).copied().unwrap_or(0) as i64) << 8
                       | (b.get(5).copied().unwrap_or(0) as i64);
            (SqlVal::Int(v), 6)
        }
        6 => {
            let b: [u8;8] = get(0,8).try_into().unwrap_or([0;8]);
            (SqlVal::Int(i64::from_be_bytes(b)), 8)
        }
        7 => {
            let b: [u8;8] = get(0,8).try_into().unwrap_or([0;8]);
            (SqlVal::Real(f64::from_be_bytes(b)), 8)
        }
        8 => (SqlVal::Int(0), 0),
        9 => (SqlVal::Int(1), 0),
        t if t >= 12 && t % 2 == 0 => {
            let len = ((t - 12) / 2) as usize;
            (SqlVal::Blob(get(0, len)), len)
        }
        t if t >= 13 && t % 2 == 1 => {
            let len = ((t - 13) / 2) as usize;
            let s   = String::from_utf8_lossy(&get(0, len)).into_owned();
            (SqlVal::Text(s), len)
        }
        _ => (SqlVal::Null, 0),
    }
}

fn encode_serial(val: &SqlVal) -> (u64, Vec<u8>) {
    match val {
        SqlVal::Null    => (0, vec![]),
        SqlVal::Int(v)  => {
            let v = *v;
            if v == 0 { return (8, vec![]); }
            if v == 1 { return (9, vec![]); }
            if v >= i8::MIN as i64  && v <= i8::MAX as i64  { return (1, vec![v as i8 as u8]); }
            if v >= i16::MIN as i64 && v <= i16::MAX as i64 { return (2, (v as i16).to_be_bytes().to_vec()); }
            if v >= i32::MIN as i64 && v <= i32::MAX as i64 { return (4, (v as i32).to_be_bytes().to_vec()); }
            (6, v.to_be_bytes().to_vec())
        }
        SqlVal::Real(v) => (7, v.to_be_bytes().to_vec()),
        SqlVal::Text(s) => {
            let b = s.as_bytes();
            (b.len() as u64 * 2 + 13, b.to_vec())
        }
        SqlVal::Blob(b) => (b.len() as u64 * 2 + 12, b.clone()),
    }
}

fn build_record_payload(values: &[SqlVal]) -> Vec<u8> {
    let mut types  = Vec::new();
    let mut bodies = Vec::new();
    for v in values {
        let (t, b) = encode_serial(v);
        types.push(t);
        bodies.extend_from_slice(&b);
    }

    // Build header: first encode all type varints
    let mut hdr_body = Vec::new();
    for t in &types { hdr_body.extend_from_slice(&write_varint(*t)); }
    // Header size includes the varint that stores header size itself.
    let total_hdr_content = hdr_body.len();
    let mut hdr_size = total_hdr_content + 1;
    loop {
        let len = write_varint(hdr_size as u64).len();
        let next = total_hdr_content + len;
        if next == hdr_size { break; }
        hdr_size = next;
    }
    let hdr_size_varint = write_varint(hdr_size as u64);

    let mut record = Vec::new();
    record.extend_from_slice(&hdr_size_varint);
    record.extend_from_slice(&hdr_body);
    record.extend_from_slice(&bodies);

    record
}

// ══════════════════════════════════════════════════════════════════════════════
// SQL helpers
// ══════════════════════════════════════════════════════════════════════════════

pub(crate) fn extract_table_name(sql: &str) -> Option<String> {
    // Match: CREATE TABLE [IF NOT EXISTS] <name> (
    let lower = sql.to_ascii_lowercase();
    let after_create = lower.find("create table")?;
    let rest = sql[after_create + 12..].trim_start();
    let rest = if rest.to_ascii_lowercase().starts_with("if not exists") {
        rest[13..].trim_start()
    } else { rest };
    let end = rest.find(|c: char| c.is_whitespace() || c == '(').unwrap_or(rest.len());
    Some(rest[..end].trim_matches('"').to_owned())
}

pub(crate) fn extract_column_names(sql: &str) -> Vec<String> {
    let start = sql.find('(').map(|i| i + 1).unwrap_or(0);
    let end   = sql.rfind(')').unwrap_or(sql.len());
    if start >= end { return Vec::new(); }

    sql[start..end]
        .split(',')
        .filter_map(|col| {
            let t = col.trim();
            if t.is_empty() { return None; }
            let first = t.split_whitespace().next()?;
            let name  = first.trim_matches('"');
            // Skip table-level constraints
            let low = name.to_ascii_lowercase();
            if ["constraint","primary","unique","check","foreign"].contains(&low.as_str()) {
                return None;
            }
            Some(name.to_owned())
        })
        .collect()
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn btree_depth_and_check(db: &Db, page_no: usize, ho: usize) -> Result<usize> {
        if page_no == 0 || page_no > db.pages.len() {
            return Err(GeoError::Sqlite(format!("invalid page reference {page_no}")));
        }
        let page = &db.pages[page_no - 1];
        if page.len() < ho + 8 {
            return Err(GeoError::Sqlite(format!("page {page_no} too short")));
        }
        let page_type = page[ho];
        let n_cells = u16::from_be_bytes([page[ho + 3], page[ho + 4]]) as usize;
        match page_type {
            0x0D => Ok(1),
            0x05 => {
                if page.len() < ho + 12 {
                    return Err(GeoError::Sqlite(format!("interior page {page_no} too short")));
                }
                let mut depths = Vec::with_capacity(n_cells + 1);
                let cell_arr_start = ho + 12;
                for i in 0..n_cells {
                    let ptr_off = cell_arr_start + i * 2;
                    if ptr_off + 2 > page.len() {
                        return Err(GeoError::Sqlite(format!("bad cell pointer array on page {page_no}")));
                    }
                    let cell_off = u16::from_be_bytes([page[ptr_off], page[ptr_off + 1]]) as usize;
                    if cell_off + 4 > page.len() {
                        return Err(GeoError::Sqlite(format!("bad cell offset on page {page_no}")));
                    }
                    let child = u32::from_be_bytes([
                        page[cell_off],
                        page[cell_off + 1],
                        page[cell_off + 2],
                        page[cell_off + 3],
                    ]) as usize;
                    depths.push(btree_depth_and_check(db, child, 0)?);
                }
                let right = u32::from_be_bytes([
                    page[ho + 8],
                    page[ho + 9],
                    page[ho + 10],
                    page[ho + 11],
                ]) as usize;
                depths.push(btree_depth_and_check(db, right, 0)?);
                let first = *depths.first().ok_or_else(|| {
                    GeoError::Sqlite(format!("interior page {page_no} has no children"))
                })?;
                if depths.iter().any(|d| *d != first) {
                    return Err(GeoError::Sqlite(format!(
                        "child page depth differs at page {page_no}"
                    )));
                }
                Ok(first + 1)
            }
            other => Err(GeoError::Sqlite(format!("unexpected page type 0x{other:02X}"))),
        }
    }

    #[test]
    fn create_and_insert() {
        let mut db = Db::new_empty();
        db.create_table("CREATE TABLE foo (id INTEGER, name TEXT)").unwrap();
        db.insert("foo", vec![SqlVal::Int(1), SqlVal::Text("hello".into())]).unwrap();
        db.insert("foo", vec![SqlVal::Int(2), SqlVal::Text("world".into())]).unwrap();
        let rows = db.select_all("foo").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][1].as_str(), Some("hello"));
        assert_eq!(rows[1][0].as_i64(), Some(2));
    }

    #[test]
    fn roundtrip_bytes() {
        let mut db = Db::new_empty();
        db.create_table("CREATE TABLE nums (v REAL)").unwrap();
        db.insert("nums", vec![SqlVal::Real(3.14)]).unwrap();
        let bytes = db.to_bytes();
        let db2   = Db::from_bytes(bytes).unwrap();
        let rows  = db2.select_all("nums").unwrap();
        assert_eq!(rows.len(), 1);
        assert!((rows[0][0].as_f64().unwrap() - 3.14).abs() < 1e-9);
    }

    #[test]
    fn roundtrip_large_blob_overflow_payload() {
        let mut db = Db::new_empty();
        db.create_table("CREATE TABLE blobs (b BLOB)").unwrap();

        let blob: Vec<u8> = (0..200_000usize).map(|i| (i % 251) as u8).collect();
        db.insert("blobs", vec![SqlVal::Blob(blob.clone())]).unwrap();

        let bytes = db.to_bytes();
        let db2 = Db::from_bytes(bytes).unwrap();
        let rows = db2.select_all("blobs").unwrap();

        assert_eq!(rows.len(), 1);
        let got = rows[0][0].as_blob().unwrap();
        assert_eq!(got.len(), blob.len());
        assert_eq!(got, blob.as_slice());
    }

    #[test]
    fn build_cell_large_payload_includes_overflow_pointer() {
        let mut db = Db::new_empty();
        let huge = SqlVal::Blob(vec![7u8; 180_000]);
        let cell = db.build_leaf_cell_with_overflow(1, &[huge]).unwrap();

        // payload varint + rowid varint + local payload + overflow page pointer
        let (payload_size, n1) = read_varint(&cell, 0);
        let (_rowid, n2) = read_varint(&cell, n1);
        let payload_size = payload_size as usize;
        let local = Db::table_leaf_local_payload(payload_size, db.page_size);
        assert!(payload_size > local);
        assert!(cell.len() >= n1 + n2 + local + 4);

        let ptr_off = n1 + n2 + local;
        let first_overflow = u32::from_be_bytes([
            cell[ptr_off],
            cell[ptr_off + 1],
            cell[ptr_off + 2],
            cell[ptr_off + 3],
        ]) as usize;
        assert!(first_overflow > 0);
        assert!(first_overflow <= db.pages.len());
    }

    #[test]
    fn many_inserts_across_page_splits_remain_visible() {
        let mut db = Db::new_empty();
        db.create_table("CREATE TABLE items (id INTEGER, name TEXT)").unwrap();

        for i in 0..5000i64 {
            db.insert(
                "items",
                vec![
                    SqlVal::Int(i),
                    SqlVal::Text(format!("item_{i:05}_{}", "x".repeat(96))),
                ],
            )
            .unwrap();
        }

        let rows = db.select_all("items").unwrap();
        assert_eq!(rows.len(), 5000);
        assert_eq!(rows.first().and_then(|r| r.get(0)).and_then(|v| v.as_i64()), Some(0));
        assert_eq!(rows.last().and_then(|r| r.get(0)).and_then(|v| v.as_i64()), Some(4999));

        let root = db.table_meta("items").unwrap().root_page;
        btree_depth_and_check(&db, root, 0).unwrap();

        // Also verify after serialise + parse roundtrip.
        let db2 = Db::from_bytes(db.to_bytes()).unwrap();
        let rows2 = db2.select_all("items").unwrap();
        assert_eq!(rows2.len(), 5000);
        assert_eq!(rows2.last().and_then(|r| r.get(0)).and_then(|v| v.as_i64()), Some(4999));

        let root2 = db2.table_meta("items").unwrap().root_page;
        btree_depth_and_check(&db2, root2, 0).unwrap();
    }

    #[test]
    fn extract_name() {
        assert_eq!(extract_table_name("CREATE TABLE gpkg_contents (id INTEGER)"), Some("gpkg_contents".into()));
        assert_eq!(extract_table_name("create table if not exists foo (x text)"), Some("foo".into()));
    }

    #[test]
    fn extract_cols() {
        let cols = extract_column_names("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, val REAL)");
        assert_eq!(cols, vec!["id", "name", "val"]);
    }
}
