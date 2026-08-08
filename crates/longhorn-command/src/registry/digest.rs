//! Registry digest materialization.

use serde::Serialize;

use crate::{
    CommandCapabilityDefinition, CommandContextDefinition, CommandDefinition, CommandLimits,
    CommandRegistryDigest, CommandRegistryError, CommandRegistryErrorCode, error::registry_error,
};

use super::CommandRegistry;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistryDigestMaterial<'registry> {
    pub(crate) limits: CommandLimits,
    pub(crate) commands: &'registry [CommandDefinition],
    pub(crate) contexts: &'registry [CommandContextDefinition],
    pub(crate) capabilities: &'registry [CommandCapabilityDefinition],
}

pub(crate) fn compute_digest(
    registry: &CommandRegistry,
) -> Result<CommandRegistryDigest, CommandRegistryError> {
    serde_json::to_vec(&RegistryDigestMaterial {
        limits: registry.limits,
        commands: &registry.commands,
        contexts: &registry.contexts,
        capabilities: &registry.capabilities,
    })
    .map(|bytes| CommandRegistryDigest::from_bytes(&bytes))
    .map_err(|error| {
        registry_error(
            CommandRegistryErrorCode::DigestEncoding,
            format!("could not encode command registry digest material: {error}"),
        )
    })
}
