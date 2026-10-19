//! Bounded, redacted support logs for desktop apps.
//!
//! This crate implements Longhorn contract 025. It holds three bounded logs:
//! a fixed-capacity in-memory [`RingLog`], a size-bounded [`RotatingLog`] on
//! disk, and the [`bound_field`] and [`bounded_tail`] helpers for text
//! fields and process output.
//!
//! Every text goes through a caller-supplied [`longhorn_redact::Redactor`]
//! before any cut, so a cut never exposes part of a secret. No public path
//! bounds raw text.
//!
//! The caller owns every product choice: capacities, bounds, the directory,
//! file names, record fields, and any process-wide instance.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod ring;
mod rotating;
mod text;
mod time;

pub use ring::{Level, RingLog, RingRecord};
pub use rotating::{JsonRecord, LogStats, RotatingLog, RotatingLogConfig};
pub use text::{OMITTED_MARKER, TailBounds, bound_field, bounded_tail};
pub use time::utc_timestamp;
