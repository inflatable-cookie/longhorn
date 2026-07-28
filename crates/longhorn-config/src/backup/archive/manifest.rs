use std::collections::{BTreeMap, BTreeSet};

use crate::{
    BackupManifest, BackupPayloadPath, BackupSourceState, Sha256Digest,
    backup::types::parse_utc_timestamp,
};

use super::BackupArchiveError;

#[derive(Clone, Debug)]
pub(super) struct DeclaredPayload {
    pub(super) path: BackupPayloadPath,
    pub(super) byte_length: u64,
    pub(super) sha256: Sha256Digest,
}

pub(super) fn validate_manifest(
    manifest: &BackupManifest,
) -> Result<BTreeMap<String, DeclaredPayload>, BackupArchiveError> {
    parse_utc_timestamp(manifest.created_at()).map_err(|error| invariant(error.to_string()))?;

    let mut groups = BTreeSet::new();
    let mut previous_group = None;
    for group in manifest.consistency_groups() {
        if previous_group.is_some_and(|previous| previous >= group.id()) {
            return Err(invariant(
                "consistency groups are not in stable unique order",
            ));
        }
        if !groups.insert(group.id()) {
            return Err(invariant(format!(
                "duplicate consistency group {}",
                group.id()
            )));
        }
        previous_group = Some(group.id());
    }
    if groups.is_empty() && !manifest.domains().is_empty() {
        return Err(invariant("manifest has no consistency group"));
    }

    let mut domain_ids = BTreeSet::new();
    let mut declared = BTreeMap::new();
    let mut previous_domain = None;
    for domain in manifest.domains() {
        if previous_domain.is_some_and(|previous| previous >= domain.domain()) {
            return Err(invariant("domains are not in stable unique order"));
        }
        if !domain_ids.insert(domain.domain()) {
            return Err(invariant(format!("duplicate domain {}", domain.domain())));
        }
        if !groups.contains(domain.consistency_group()) {
            return Err(invariant(format!(
                "domain {} names missing consistency group {}",
                domain.domain(),
                domain.consistency_group()
            )));
        }
        validate_source_shape(domain)?;
        for payload in domain.payloads() {
            validate_payload_namespace(domain.adapter(), domain.domain(), payload.path())?;
            let path = payload.path().as_str().to_owned();
            if declared
                .insert(
                    path.clone(),
                    DeclaredPayload {
                        path: payload.path().clone(),
                        byte_length: payload.byte_length(),
                        sha256: payload.sha256().clone(),
                    },
                )
                .is_some()
            {
                return Err(invariant(format!("duplicate payload path {path}")));
            }
        }
        previous_domain = Some(domain.domain());
    }

    let mut previous_exclusion = None;
    for exclusion in manifest.exclusions() {
        if previous_exclusion.is_some_and(|previous| previous >= exclusion.domain()) {
            return Err(invariant("exclusions are not in stable unique order"));
        }
        if domain_ids.contains(exclusion.domain()) {
            return Err(invariant(format!(
                "domain {} is both included and excluded",
                exclusion.domain()
            )));
        }
        previous_exclusion = Some(exclusion.domain());
    }

    Ok(declared)
}

fn validate_source_shape(domain: &crate::BackupManifestDomain) -> Result<(), BackupArchiveError> {
    let valid = match domain.state() {
        BackupSourceState::Absent => {
            domain.source_schema_version().is_none()
                && domain.source_issue().is_none()
                && domain.payloads().is_empty()
        }
        BackupSourceState::Present => {
            domain.source_schema_version().is_some()
                && domain.source_issue().is_none()
                && !domain.payloads().is_empty()
        }
        BackupSourceState::SourcePreserved => {
            domain.source_issue().is_some() && !domain.payloads().is_empty()
        }
    };
    if valid {
        Ok(())
    } else {
        Err(invariant(format!(
            "domain {} has inconsistent source evidence",
            domain.domain()
        )))
    }
}

fn validate_payload_namespace(
    adapter: &str,
    domain: &longhorn_core::DomainId,
    path: &BackupPayloadPath,
) -> Result<(), BackupArchiveError> {
    let expected = if adapter == "longhorn-json-v1" {
        format!("longhorn/domains/{domain}.json")
    } else {
        format!("longhorn/adapters/{domain}/")
    };
    let valid = if adapter == "longhorn-json-v1" {
        path.as_str() == expected
    } else {
        path.as_str().starts_with(&expected) && path.as_str().len() > expected.len()
    };
    if valid {
        Ok(())
    } else {
        Err(invariant(format!(
            "domain {domain} payload {} does not match adapter {adapter}",
            path.as_str()
        )))
    }
}

fn invariant(detail: impl Into<String>) -> BackupArchiveError {
    BackupArchiveError::ManifestInvariant {
        detail: detail.into(),
    }
}
