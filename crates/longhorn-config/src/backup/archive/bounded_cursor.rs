use std::io::{self, Cursor, Seek, SeekFrom, Write};

pub(super) struct BoundedCursor {
    inner: Cursor<Vec<u8>>,
    limit: usize,
}

impl BoundedCursor {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            inner: Cursor::new(Vec::new()),
            limit,
        }
    }

    pub(super) fn into_inner(self) -> Vec<u8> {
        self.inner.into_inner()
    }
}

impl Write for BoundedCursor {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let position = usize::try_from(self.inner.position())
            .map_err(|_| io::Error::other("archive position exceeds usize"))?;
        let end = position
            .checked_add(buffer.len())
            .ok_or_else(|| io::Error::other("archive byte position overflow"))?;
        if end > self.limit {
            return Err(io::Error::other("encoded archive exceeds byte limit"));
        }
        self.inner.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for BoundedCursor {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}
