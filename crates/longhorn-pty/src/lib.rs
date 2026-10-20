//! Bounded pseudo-terminal sessions for desktop hosts.
//!
//! This crate implements Longhorn contract 028. A session starts one caller
//! program with the supplied arguments, environment, working directory and
//! terminal size. It streams output as bounded byte chunks and accepts input,
//! resize and close operations. It does not add environment variables, encode
//! output for a renderer, choose allowed programs or assign session ids.

#![deny(missing_docs)]

mod api;

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "illumos",
))]
mod unix;

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "illumos",
)))]
mod unsupported;

pub use api::{
    CLOSE_GRACE_PERIOD, CommandSpec, DEFAULT_SIZE, ExitResult, Failure, MAX_BUFFERED_OUTPUT_BYTES,
    OUTPUT_CHUNK_BYTES, OUTPUT_QUEUE_CAPACITY, PtyEvent, PtySession, PtySize,
};
