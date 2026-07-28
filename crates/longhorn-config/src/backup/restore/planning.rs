use std::{collections::BTreeSet, io};

use cap_std::{ambient_authority, fs::Dir};
use longhorn_core::SchemaVersion;
use serde_json::json;

use crate::{ConfigStore, DomainLocation, ResolvedFile, Sha256Digest};

use super::types::{
    PlannedTarget, PreparedTarget, RestoreAction, RestoreChoices, RestoreConflictChoice,
    RestoreCurrentEvidence, RestoreDomainCompatibility, RestoreDomainInspection, RestoreInspection,
    RestorePlan, RestorePlanEntry, RestorePlanError, RestorePlanReceipt,
};

pub(crate) fn plan(
    store: &ConfigStore,
    inspection: &RestoreInspection,
    choices: &RestoreChoices,
) -> Result<RestorePlan, RestorePlanError> {
    if !inspection.identity.is_compatible() {
        return Err(RestorePlanError::IdentityMismatch);
    }

    validate_choices(inspection, choices)?;
    let selected = choices
        .choices
        .values()
        .filter(|choice| **choice == RestoreConflictChoice::UseArchive)
        .count();
    if selected == 0 {
        return Err(RestorePlanError::EmptySelection);
    }

    let mut entries = Vec::with_capacity(inspection.domains.len());
    let mut targets = Vec::with_capacity(selected);
    let mut receipt = RestorePlanReceipt {
        selected,
        skipped: inspection.domains.len() - selected,
        creates: 0,
        replaces: 0,
        deletes: 0,
        migrations: 0,
        unchanged: 0,
    };

    for report in &inspection.domains {
        let domain = report.domain().clone();
        let choice = choices.get(&domain).expect("choice coverage was validated");
        if choice == RestoreConflictChoice::KeepCurrent {
            entries.push(RestorePlanEntry {
                domain,
                choice,
                action: None,
                current: None,
            });
            continue;
        }
        if !report.compatibility().is_restorable() {
            return Err(RestorePlanError::DomainNotRestorable {
                domain,
                compatibility: report.compatibility().clone(),
            });
        }

        let descriptor = store
            .registered_descriptor(&domain)
            .expect("restorable inspection has a registered descriptor");
        let DomainLocation::File(file) = store.roots.resolve(descriptor) else {
            unreachable!("restorable inspection has an ordinary file target");
        };
        let current =
            read_current_evidence(&file).map_err(|error| RestorePlanError::CurrentReadFailed {
                domain: domain.clone(),
                path: file.full_path().to_path_buf(),
                detail: error.to_string(),
            })?;
        let prepared = inspection
            .prepared
            .get(&domain)
            .expect("restorable inspection retains a prepared target");
        let target_sha256 = prepared.bytes.as_deref().map(Sha256Digest::from_bytes);
        let action = derive_action(report, &current, prepared, target_sha256.as_ref());
        increment_action(&mut receipt, action);
        entries.push(RestorePlanEntry {
            domain: domain.clone(),
            choice,
            action: Some(action),
            current: Some(current.clone()),
        });
        targets.push(PlannedTarget {
            domain,
            action,
            current,
            path: file.full_path().to_path_buf(),
            target_schema_version: prepared.schema_version,
            target_sha256,
        });
    }

    let digest = plan_digest(&inspection.archive_sha256, &entries, &targets);
    Ok(RestorePlan {
        archive_sha256: inspection.archive_sha256.clone(),
        digest,
        entries,
        targets,
        receipt,
    })
}

fn validate_choices(
    inspection: &RestoreInspection,
    choices: &RestoreChoices,
) -> Result<(), RestorePlanError> {
    let manifest = inspection
        .domains
        .iter()
        .map(|domain| domain.domain().clone())
        .collect::<BTreeSet<_>>();
    let supplied = choices.choices.keys().cloned().collect::<BTreeSet<_>>();
    let missing = manifest.difference(&supplied).cloned().collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(RestorePlanError::MissingChoices { domains: missing });
    }
    let unexpected = supplied.difference(&manifest).cloned().collect::<Vec<_>>();
    if !unexpected.is_empty() {
        return Err(RestorePlanError::UnexpectedChoices {
            domains: unexpected,
        });
    }
    Ok(())
}

fn derive_action(
    report: &RestoreDomainInspection,
    current: &RestoreCurrentEvidence,
    prepared: &PreparedTarget,
    target_sha256: Option<&Sha256Digest>,
) -> RestoreAction {
    match (&prepared.bytes, current) {
        (None, RestoreCurrentEvidence::Absent) => RestoreAction::Unchanged,
        (None, RestoreCurrentEvidence::Present { .. }) => RestoreAction::Delete,
        (
            Some(bytes),
            RestoreCurrentEvidence::Present {
                byte_length,
                sha256,
            },
        ) if *byte_length == bytes.len() as u64 && Some(sha256) == target_sha256 => {
            RestoreAction::Unchanged
        }
        (Some(_), _)
            if matches!(
                report.compatibility(),
                RestoreDomainCompatibility::MigrationRequired { .. }
            ) =>
        {
            RestoreAction::Migrate
        }
        (Some(_), RestoreCurrentEvidence::Absent) => RestoreAction::Create,
        (Some(_), RestoreCurrentEvidence::Present { .. }) => RestoreAction::Replace,
    }
}

fn increment_action(receipt: &mut RestorePlanReceipt, action: RestoreAction) {
    match action {
        RestoreAction::Create => receipt.creates += 1,
        RestoreAction::Replace => receipt.replaces += 1,
        RestoreAction::Delete => receipt.deletes += 1,
        RestoreAction::Migrate => receipt.migrations += 1,
        RestoreAction::Unchanged => receipt.unchanged += 1,
    }
}

pub(super) fn read_current_evidence(file: &ResolvedFile) -> io::Result<RestoreCurrentEvidence> {
    let directory = Dir::open_ambient_dir(file.root(), ambient_authority())?;
    match directory.open(file.relative_path().as_path()) {
        Ok(input) => {
            let (byte_length, sha256) = Sha256Digest::from_reader(input)?;
            Ok(RestoreCurrentEvidence::Present {
                byte_length,
                sha256,
            })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(RestoreCurrentEvidence::Absent),
        Err(error) => Err(error),
    }
}

fn plan_digest(
    archive_sha256: &Sha256Digest,
    entries: &[RestorePlanEntry],
    targets: &[PlannedTarget],
) -> Sha256Digest {
    let choices = entries
        .iter()
        .map(|entry| {
            json!({
                "domain": entry.domain.as_str(),
                "choice": choice_name(entry.choice),
                "action": entry.action.map(action_name),
                "current": entry.current.as_ref().map(evidence_json),
            })
        })
        .collect::<Vec<_>>();
    let target_evidence = targets
        .iter()
        .map(|target| {
            json!({
                "domain": target.domain.as_str(),
                "schemaVersion": target.target_schema_version.map(SchemaVersion::get),
                "sha256": target.target_sha256.as_ref().map(Sha256Digest::as_str),
            })
        })
        .collect::<Vec<_>>();
    let canonical = serde_json::to_vec(&json!({
        "archiveSha256": archive_sha256.as_str(),
        "choices": choices,
        "targets": target_evidence,
    }))
    .expect("restore plan canonical form is serializable");
    Sha256Digest::from_bytes(&canonical)
}

const fn choice_name(choice: RestoreConflictChoice) -> &'static str {
    match choice {
        RestoreConflictChoice::UseArchive => "use-archive",
        RestoreConflictChoice::KeepCurrent => "keep-current",
    }
}

const fn action_name(action: RestoreAction) -> &'static str {
    match action {
        RestoreAction::Create => "create",
        RestoreAction::Replace => "replace",
        RestoreAction::Delete => "delete",
        RestoreAction::Migrate => "migrate",
        RestoreAction::Unchanged => "unchanged",
    }
}

fn evidence_json(evidence: &RestoreCurrentEvidence) -> serde_json::Value {
    match evidence {
        RestoreCurrentEvidence::Absent => json!({"state": "absent"}),
        RestoreCurrentEvidence::Present {
            byte_length,
            sha256,
        } => json!({
            "state": "present",
            "byteLength": byte_length,
            "sha256": sha256.as_str(),
        }),
    }
}
