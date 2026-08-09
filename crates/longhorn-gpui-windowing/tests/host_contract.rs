//! Contract 020 host-boundary conformance for the GPUI backend.
//!
//! One module per requirement in "What A Host Must Provide". Requirements the
//! GPUI host cannot meet are asserted as refusals, not skipped.

mod host_contract {
    mod displays;
    mod lifecycle;
    mod placement;
    mod support;
    mod windows;
}
