//! Read wrappers that tag their errors (contract 027).
//!
//! The bound counts bytes as they are produced, never declared sizes. A
//! reader past its budget fails with [`ByteBudgetExceeded`] inside an
//! `io::Error`, so the caller can tell the bound apart from a damaged archive.
//! A [`SourceReader`] tags the host's own read and seek failures with
//! [`SourceFault`], so a host fault stays apart from undecodable data.

use std::{
    fmt,
    io::{self, Read, Seek, SeekFrom},
};

/// The payload of the `io::Error` a [`BudgetReader`] returns past its limit.
#[derive(Debug)]
pub(crate) struct ByteBudgetExceeded;

impl fmt::Display for ByteBudgetExceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the decompressed byte bound was exceeded")
    }
}

impl std::error::Error for ByteBudgetExceeded {}

/// Passes bytes through and refuses once more than its budget has passed.
pub(crate) struct BudgetReader<R> {
    inner: R,
    remaining: u64,
}

impl<R> BudgetReader<R> {
    pub(crate) fn new(inner: R, budget: u64) -> Self {
        Self {
            inner,
            remaining: budget,
        }
    }
}

impl<R: Read> Read for BudgetReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        let read_bytes = read as u64;
        if read_bytes > self.remaining {
            self.remaining = 0;
            return Err(io::Error::other(ByteBudgetExceeded));
        }
        self.remaining -= read_bytes;
        Ok(read)
    }
}

/// The payload of the `io::Error` a [`SourceReader`] returns when the host
/// read or seek itself failed.
#[derive(Debug)]
pub(crate) struct SourceFault(io::Error);

impl fmt::Display for SourceFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for SourceFault {}

/// Passes reads and seeks through, tagging their failures as [`SourceFault`].
/// `Interrupted` passes untagged, so callers still retry it.
pub(crate) struct SourceReader<R>(R);

impl<R> SourceReader<R> {
    pub(crate) fn new(inner: R) -> Self {
        Self(inner)
    }
}

fn tag(error: io::Error) -> io::Error {
    if error.kind() == io::ErrorKind::Interrupted {
        error
    } else {
        io::Error::new(error.kind(), SourceFault(error))
    }
}

impl<R: Read> Read for SourceReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0.read(buffer).map_err(tag)
    }
}

impl<R: Seek> Seek for SourceReader<R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.0.seek(position).map_err(tag)
    }
}
