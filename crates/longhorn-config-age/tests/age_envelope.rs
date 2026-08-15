//! Binary age v1 backup envelope conformance.

#[path = "age_envelope/failures.rs"]
mod failures;
#[path = "age_envelope/passphrase.rs"]
mod passphrase;
#[path = "age_envelope/recipient.rs"]
mod recipient;
#[path = "age_envelope/retention.rs"]
mod retention;
#[path = "age_envelope/rotation.rs"]
mod rotation;

#[path = "age_envelope/store.rs"]
mod store;
#[path = "age_envelope/support.rs"]
mod support;
