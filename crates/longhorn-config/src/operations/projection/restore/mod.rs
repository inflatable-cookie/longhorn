//! Restore operation projection conversions.

mod impls;
mod map;

pub(crate) use map::{
    action_id, choice, compatibility, current_evidence, domain_ids, execution_stage,
    failure_terminal, identity_status, location_id, participation, source_state,
};
