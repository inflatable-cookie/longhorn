//! A read budget over decompressed bytes (contract 027).
//!
//! The bound counts bytes as they are produced, never declared sizes. A
//! reader past its budget fails with [`ByteBudgetExceeded`] inside an
//! `io::Error`, so the caller can tell the bound apart from a damaged archive.

use std::{
    fmt,
    io::{self, Read},
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
