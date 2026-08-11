//! Layout vocabulary and state: schema definitions, regions, panels, sizing.
//!
//! Absorbed from the former `longhorn-layout` crate by Card 179. The module
//! boundary is what states where layout vocabulary ends; it no longer needs a
//! Cargo manifest to say so.

/// Compatibility version of the serialized layout protocol.
pub const LAYOUT_PROTOCOL_VERSION: u32 = 1;

/// Registered schema, region, panel and sizing-slot definitions.
pub mod definition;
/// Bounded counts for layout state.
pub mod limits;
/// Durable layout state: regions, sizing slots and panel instances.
pub mod model;
/// Expected-revision layout mutation protocol and engine.
pub mod mutation;
/// Fixed-point sizing ratios in millionths.
pub mod ratio;
/// Layout document validation and canonical normalization.
pub mod validation;
/// Region visibility projection from definitions and state.
pub mod visibility;
