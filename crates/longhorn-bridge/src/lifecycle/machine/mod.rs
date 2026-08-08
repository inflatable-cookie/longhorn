//! Pure validated connection state machine for one selected bridge host.

mod authority;
mod handlers;
mod state;
mod terminals;
mod transitions;

pub use state::BridgeConnectionMachine;
