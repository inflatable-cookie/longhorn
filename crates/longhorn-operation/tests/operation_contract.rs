//! Contract fixtures for the pure asynchronous-operation foundation.

#[path = "operation_contract/cancellation.rs"]
mod cancellation;
#[path = "operation_contract/invariants.rs"]
mod invariants;
#[path = "operation_contract/lifecycle.rs"]
mod lifecycle;
#[path = "operation_contract/loophole.rs"]
mod loophole;
#[path = "operation_contract/progress.rs"]
mod progress;
#[path = "operation_contract/protocol.rs"]
mod protocol;
#[path = "operation_contract/retention.rs"]
mod retention;
#[path = "operation_contract/retry_teardown.rs"]
mod retry_teardown;
#[path = "operation_contract/soundcheck.rs"]
mod soundcheck;
#[path = "operation_contract/support.rs"]
mod support;
