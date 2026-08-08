//! Grouped custom-adapter restore execution.

mod prepare;
mod run;
mod support;

pub(crate) use prepare::prepare_domains;
pub(crate) use run::execute;
pub(crate) use support::{
    failure, rollback_after_failure, stage_limit_failure, validate_plan, validate_state_payload_set,
};
