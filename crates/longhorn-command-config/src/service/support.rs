//! Shared helpers and proposal types for keymap service.

use std::collections::{BTreeMap, BTreeSet};

use longhorn_command::{CommandEffectiveKeymap, CommandKeymapConflict, CommandReservedChordPolicy};

use crate::{
    CommandKeymapDiagnostic, CommandKeymapLoadOrigin, CommandKeymapMigration,
    CommandKeymapMutationResult, CommandKeymapPatch, CommandKeymapProtocolVersion,
    CommandKeymapRejection, CommandKeymapRejectionCode, CommandKeymapSnapshot, CommandKeymapState,
    RegisteredCommandKeymapDomain, protocol::override_binding_id,
};

use super::CommandKeymapServiceError;

pub(crate) fn snapshot_from_effective<P, M>(
    domain: &RegisteredCommandKeymapDomain<P, M>,
    state: CommandKeymapState,
    preset_version: longhorn_core::SchemaVersion,
    effective: CommandEffectiveKeymap,
    origin: CommandKeymapLoadOrigin,
    diagnostics: Vec<CommandKeymapDiagnostic>,
) -> CommandKeymapSnapshot
where
    P: CommandReservedChordPolicy,
    M: CommandKeymapMigration<P>,
{
    CommandKeymapSnapshot {
        protocol_version: CommandKeymapProtocolVersion::CURRENT,
        registry_generation: domain.registry().generation(),
        registry_digest: domain.registry().digest().clone(),
        state,
        active_preset_version: preset_version,
        bindings: effective.bindings().cloned().collect(),
        conflicts: effective.conflicts().cloned().collect(),
        origin,
        diagnostics,
    }
}

pub(crate) fn validate_patch(patch: &CommandKeymapPatch) -> Result<CommandKeymapPatch, String> {
    let canonical = patch.canonical();
    let removes = canonical.remove_binding_ids.iter().collect::<BTreeSet<_>>();
    if removes.len() != canonical.remove_binding_ids.len() {
        return Err("patch contains duplicate remove binding ids".into());
    }
    let mut upserts = BTreeSet::new();
    for directive in &canonical.upsert_overrides {
        let binding_id = override_binding_id(directive);
        if !upserts.insert(binding_id.clone()) {
            return Err(format!(
                "patch contains duplicate upsert binding id {binding_id}"
            ));
        }
        if removes.contains(&binding_id) {
            return Err(format!(
                "patch both removes and upserts binding id {binding_id}"
            ));
        }
    }
    Ok(canonical)
}

pub(crate) fn apply_patch(state: &mut CommandKeymapState, patch: &CommandKeymapPatch) {
    if let Some(preset_id) = &patch.active_preset_id {
        state.active_preset_id = preset_id.clone();
    }
    let mut overrides = if patch.clear_overrides {
        BTreeMap::new()
    } else {
        state
            .overrides
            .iter()
            .cloned()
            .map(|directive| (override_binding_id(&directive), directive))
            .collect()
    };
    for binding_id in &patch.remove_binding_ids {
        overrides.remove(binding_id);
    }
    for directive in &patch.upsert_overrides {
        overrides.insert(override_binding_id(directive), directive.clone());
    }
    state.overrides = overrides.into_values().collect();
}

pub(crate) fn rejection(
    code: CommandKeymapRejectionCode,
    detail: impl Into<String>,
) -> CommandKeymapRejection {
    CommandKeymapRejection {
        code,
        detail: detail.into(),
    }
}

pub(crate) enum Proposal {
    Accepted {
        state: CommandKeymapState,
        snapshot: CommandKeymapSnapshot,
    },
    Rejected {
        rejection: CommandKeymapRejection,
        conflicts: Vec<CommandKeymapConflict>,
    },
}

pub(crate) struct AcceptedCommit {
    pub(crate) previous_revision: crate::CommandKeymapRevision,
    pub(crate) origin: CommandKeymapLoadOrigin,
    pub(crate) diagnostics: Vec<CommandKeymapDiagnostic>,
}

pub(crate) enum CommitAbort {
    Result(Box<CommandKeymapMutationResult>),
    Operational(CommandKeymapServiceError),
}
