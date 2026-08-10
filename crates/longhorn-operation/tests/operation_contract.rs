//! Contract fixtures for the pure asynchronous-operation foundation.

#[path = "operation_contract/cancellation.rs"]
mod cancellation;
#[path = "operation_contract/invariants.rs"]
mod invariants;
#[path = "operation_contract/lifecycle.rs"]
mod lifecycle;
#[path = "operation_contract/long_running_scan.rs"]
mod long_running_scan;
#[path = "operation_contract/progress.rs"]
mod progress;
#[path = "operation_contract/protocol.rs"]
mod protocol;
#[path = "operation_contract/registration_order_queue.rs"]
mod registration_order_queue;
#[path = "operation_contract/retention.rs"]
mod retention;
#[path = "operation_contract/retry_teardown.rs"]
mod retry_teardown;
#[path = "operation_contract/support.rs"]
mod support;
