//! Storage transition and legacy-discovery conformance. The pure layout,
//! identity, and locator suites moved with `longhorn-storage-layout`.

#[path = "storage_layout/transition.rs"]
mod transition;
#[path = "storage_layout/transition_sqlite.rs"]
mod transition_sqlite;
#[path = "storage_layout/transition_support.rs"]
mod transition_support;
