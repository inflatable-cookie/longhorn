//! Renders already-gathered facts as one redacted diagnostics bundle.
//!
//! This crate implements Longhorn contract 026. [`render`] is a pure
//! function: it takes the facts a desktop app has already gathered, lays them
//! out as one support text in sections, and redacts the whole text last with
//! the app's [`longhorn_redact::Redactor`]. The returned [`Bundle`] carries
//! the redacted text and how many `[redacted]` markers it holds. No public
//! path returns text that has not been redacted.
//!
//! The crate reads no filesystem, no network, no credentials and no app
//! state, and it owns no product text. The title, the redaction statement,
//! the section list and order, the headings, the labels and the
//! empty-section sentences are all caller values.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bundle;

pub use bundle::{Bundle, Fact, Facts, Section, render};
