//! Projects Longhorn's domains into Poodle specs.
//!
//! The Rust sibling of `longhorn-poodle-svelte`, and deliberately **not**
//! named for a renderer. `poodle-specs` is Poodle's shared Rust contract
//! layer, and two adapters already consume it — `poodle-gpui` and
//! `poodle-jetstream` — so a projection that emits specs serves both and
//! anything after them:
//!
//! ```text
//! Longhorn domains -> longhorn-poodle -> poodle-specs
//!                                           |-> poodle-gpui      -> gpui
//!                                           `-> poodle-jetstream -> UiTree
//! ```
//!
//! Nothing here renders, so nothing here depends on a UI framework. That is
//! the same line `poodle-gpui` itself draws: it emits GPUI-shaped plain data
//! and only a preview binary builds real elements.
//!
//! # Dependency direction
//!
//! Longhorn depends on Poodle. Poodle holds no reference to Longhorn, and
//! this crate must not become the exception. Where a Longhorn concept has no
//! spec to land in, the gap is raised in Poodle rather than worked around by
//! forking a primitive.

mod notifications;

pub use notifications::{ToneMapping, project_notification, project_notifications, tone_for};
