//! Byte-range IO abstraction for COPC random-access reads.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::Result;
#[cfg(feature = "copc-http")]
use crate::Error;

/// Random-access byte source used by COPC readers.
pub trait ByteRangeSource {
    /// Return total byte length of the source.
    fn len(&mut self) -> Result<u64>;

    /// Read exactly `buf.len()` bytes at `offset`.
    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()>;

    /// Read `size` bytes at `offset` into a new vector.
    fn read_range(&mut self, offset: u64, size: usize) -> Result<Vec<u8>> {
        let mut out = vec![0u8; size];
        self.read_exact_at(offset, &mut out)?;
        Ok(out)
    }
}

impl<T: ByteRangeSource + ?Sized> ByteRangeSource for &mut T {
    fn len(&mut self) -> Result<u64> {
        (**self).len()
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        (**self).read_exact_at(offset, buf)
    }
}

impl ByteRangeSource for std::fs::File {
    fn len(&mut self) -> Result<u64> {
        let cur = self.stream_position()?;
        let end = self.seek(SeekFrom::End(0))?;
        self.seek(SeekFrom::Start(cur))?;
        Ok(end)
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.seek(SeekFrom::Start(offset))?;
        self.read_exact(buf)?;
        Ok(())
    }
}

impl<R: Read + Seek> ByteRangeSource for std::io::BufReader<R> {
    fn len(&mut self) -> Result<u64> {
        let cur = self.stream_position()?;
        let end = self.seek(SeekFrom::End(0))?;
        self.seek(SeekFrom::Start(cur))?;
        Ok(end)
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.seek(SeekFrom::Start(offset))?;
        self.read_exact(buf)?;
        Ok(())
    }
}

impl<T: AsRef<[u8]>> ByteRangeSource for std::io::Cursor<T> {
    fn len(&mut self) -> Result<u64> {
        Ok(self.get_ref().as_ref().len() as u64)
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.seek(SeekFrom::Start(offset))?;
        self.read_exact(buf)?;
        Ok(())
    }
}

/// Minimal exact-range cache wrapper over any byte-range source.
#[derive(Debug)]
pub struct CachedRangeSource<S: ByteRangeSource> {
    inner: S,
    max_entries: usize,
    cache: VecDeque<(u64, Vec<u8>)>,
}

impl<S: ByteRangeSource> CachedRangeSource<S> {
    /// Build a cached source with a fixed entry cap.
    pub fn new(inner: S, max_entries: usize) -> Self {
        Self {
            inner,
            max_entries: max_entries.max(1),
            cache: VecDeque::new(),
        }
    }

    /// Consume the wrapper and return the underlying source.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: ByteRangeSource> ByteRangeSource for CachedRangeSource<S> {
    fn len(&mut self) -> Result<u64> {
        self.inner.len()
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        if let Some((_, bytes)) = self
            .cache
            .iter()
            .find(|(off, bytes)| *off == offset && bytes.len() == buf.len())
        {
            buf.copy_from_slice(bytes);
            return Ok(());
        }

        self.inner.read_exact_at(offset, buf)?;
        if self.cache.len() >= self.max_entries {
            self.cache.pop_front();
        }
        self.cache.push_back((offset, buf.to_vec()));
        Ok(())
    }
}

impl<S: ByteRangeSource + Read> Read for CachedRangeSource<S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<S: ByteRangeSource + Seek> Seek for CachedRangeSource<S> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

/// Local-file byte-range source backend.
#[derive(Debug)]
pub struct LocalFileRangeSource {
    inner: File,
}

impl LocalFileRangeSource {
    /// Open a local file as a range source.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            inner: File::open(path)?,
        })
    }
}

impl Read for LocalFileRangeSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for LocalFileRangeSource {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl ByteRangeSource for LocalFileRangeSource {
    fn len(&mut self) -> Result<u64> {
        let cur = self.inner.stream_position()?;
        let end = self.inner.seek(SeekFrom::End(0))?;
        self.inner.seek(SeekFrom::Start(cur))?;
        Ok(end)
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.inner.seek(SeekFrom::Start(offset))?;
        self.inner.read_exact(buf)?;
        Ok(())
    }
}

#[cfg(feature = "copc-http")]
/// HTTP range-backed byte source.
#[derive(Debug)]
pub struct HttpRangeSource {
    client: reqwest::blocking::Client,
    url: String,
    len: Option<u64>,
    cursor: u64,
}

#[cfg(feature = "copc-http")]
impl HttpRangeSource {
    /// Create a new HTTP range source.
    pub fn new(url: &str) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .build()
            .map_err(|e| Error::InvalidValue {
                field: "copc.http.client",
                detail: e.to_string(),
            })?;
        Ok(Self {
            client,
            url: url.to_string(),
            len: None,
            cursor: 0,
        })
    }

    fn resolve_len(&mut self) -> Result<u64> {
        if let Some(l) = self.len {
            return Ok(l);
        }

        let resp = self
            .client
            .get(&self.url)
            .header(reqwest::header::RANGE, "bytes=0-0")
            .send()
            .map_err(|e| Error::InvalidValue {
                field: "copc.http.request",
                detail: e.to_string(),
            })?;

        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(Error::InvalidValue {
                field: "copc.http.status",
                detail: format!("unexpected status {}", resp.status()),
            });
        }

        let total = if let Some(cr) = resp.headers().get(reqwest::header::CONTENT_RANGE) {
            let s = cr.to_str().map_err(|e| Error::InvalidValue {
                field: "copc.http.content_range",
                detail: e.to_string(),
            })?;
            // expected form: bytes start-end/total
            let total_str = s.rsplit('/').next().unwrap_or("0");
            total_str.parse::<u64>().map_err(|e| Error::InvalidValue {
                field: "copc.http.content_range",
                detail: e.to_string(),
            })?
        } else {
            let head = self
                .client
                .head(&self.url)
                .send()
                .map_err(|e| Error::InvalidValue {
                    field: "copc.http.request",
                    detail: e.to_string(),
                })?;
            let cl = head
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .ok_or_else(|| Error::InvalidValue {
                    field: "copc.http.length",
                    detail: "server did not provide content range/length".to_string(),
                })?;
            let s = cl.to_str().map_err(|e| Error::InvalidValue {
                field: "copc.http.content_length",
                detail: e.to_string(),
            })?;
            s.parse::<u64>().map_err(|e| Error::InvalidValue {
                field: "copc.http.content_length",
                detail: e.to_string(),
            })?
        };

        self.len = Some(total);
        Ok(total)
    }
}

#[cfg(feature = "copc-http")]
impl ByteRangeSource for HttpRangeSource {
    fn len(&mut self) -> Result<u64> {
        self.resolve_len()
    }

    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        let end = offset.saturating_add(buf.len() as u64).saturating_sub(1);
        let range = format!("bytes={offset}-{end}");
        let mut resp = self
            .client
            .get(&self.url)
            .header(reqwest::header::RANGE, range)
            .send()
            .map_err(|e| Error::InvalidValue {
                field: "copc.http.request",
                detail: e.to_string(),
            })?;

        if !(resp.status() == reqwest::StatusCode::PARTIAL_CONTENT || resp.status().is_success()) {
            return Err(Error::InvalidValue {
                field: "copc.http.status",
                detail: format!("unexpected status {}", resp.status()),
            });
        }

        resp.read_exact(buf).map_err(Error::Io)?;
        Ok(())
    }
}

#[cfg(feature = "copc-http")]
impl Read for HttpRangeSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let offset = self.cursor;
        match self.read_exact_at(offset, buf) {
            Ok(()) => {
                self.cursor = self.cursor.saturating_add(buf.len() as u64);
                Ok(buf.len())
            }
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
        }
    }
}

#[cfg(feature = "copc-http")]
impl Seek for HttpRangeSource {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let base: i128 = match pos {
            SeekFrom::Start(v) => {
                self.cursor = v;
                return Ok(self.cursor);
            }
            SeekFrom::Current(v) => self.cursor as i128 + v as i128,
            SeekFrom::End(v) => {
                let len = self
                    .len()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                len as i128 + v as i128
            }
        };

        if base < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "negative seek",
            ));
        }
        self.cursor = base as u64;
        Ok(self.cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::{ByteRangeSource, CachedRangeSource};
    use std::io::Cursor;

    #[test]
    fn range_read_roundtrip_cursor() -> crate::Result<()> {
        let mut cur = Cursor::new(vec![1u8, 2, 3, 4, 5, 6]);
        let mut b = [0u8; 3];
        cur.read_exact_at(2, &mut b)?;
        assert_eq!(&b, &[3, 4, 5]);
        assert_eq!(cur.len()?, 6);
        Ok(())
    }

    #[test]
    fn cached_range_source_reuses_exact_range() -> crate::Result<()> {
        let cur = Cursor::new(vec![1u8, 2, 3, 4, 5, 6]);
        let mut cached = CachedRangeSource::new(cur, 2);
        let mut a = [0u8; 2];
        let mut b = [0u8; 2];
        cached.read_exact_at(2, &mut a)?;
        cached.read_exact_at(2, &mut b)?;
        assert_eq!(&a, &[3, 4]);
        assert_eq!(&b, &[3, 4]);
        Ok(())
    }
}
