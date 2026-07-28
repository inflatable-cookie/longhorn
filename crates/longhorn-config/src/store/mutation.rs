use crate::{AccessMode, ConfigDomain, DomainIssue, DomainLocation, ResolvedFile, RootKind};

use super::{
    ConfigStore, LoadOutcome, LoadedOrigin, MutationError, MutationOptions, MutationReceipt,
    MutationRefusal, document::SerializedDocument, load::load_file, publication::publish,
};

pub(super) fn mutate<D, F>(
    store: &ConfigStore,
    domain: &D,
    options: MutationOptions,
    patch: F,
) -> Result<MutationReceipt, MutationError>
where
    D: ConfigDomain,
    F: FnOnce(&mut D::Value) -> Result<(), DomainIssue>,
{
    match mutate_inner(store, domain, options, patch, false)? {
        MutationOutcome::Published(receipt) => Ok(receipt),
        MutationOutcome::Unchanged => unreachable!("immediate mutation always publishes"),
    }
}

pub(crate) fn mutate_if_changed<D, F>(
    store: &ConfigStore,
    domain: &D,
    options: MutationOptions,
    patch: F,
) -> Result<MutationOutcome, MutationError>
where
    D: ConfigDomain,
    F: FnOnce(&mut D::Value) -> Result<(), DomainIssue>,
{
    mutate_inner(store, domain, options, patch, true)
}

fn mutate_inner<D, F>(
    store: &ConfigStore,
    domain: &D,
    options: MutationOptions,
    patch: F,
    skip_unchanged: bool,
) -> Result<MutationOutcome, MutationError>
where
    D: ConfigDomain,
    F: FnOnce(&mut D::Value) -> Result<(), DomainIssue>,
{
    store
        .ensure_registered(domain)
        .map_err(MutationError::Store)?;
    let location = store.roots.resolve(domain.descriptor());
    let file = writable_file(location)?;
    let _guard = store
        .coordinator
        .acquire(options.lock_timeout)
        .map_err(MutationError::Coordination)?;
    let loaded = load_file(domain, &file);

    let mut value = match loaded {
        LoadOutcome::Ready(loaded) => match loaded.origin {
            LoadedOrigin::Default | LoadedOrigin::File => loaded.value,
            LoadedOrigin::MigratedInMemory { from, to } => {
                return Err(MutationError::Refused(
                    MutationRefusal::MigrationBackupRequired { from, to },
                ));
            }
        },
        LoadOutcome::Recovery(recovery) => {
            return Err(MutationError::Refused(MutationRefusal::Recovery(recovery)));
        }
        LoadOutcome::Unavailable(unavailable) => {
            return Err(MutationError::Refused(MutationRefusal::Unavailable {
                location: unavailable.location,
            }));
        }
    };

    let previous = skip_unchanged
        .then(|| domain.encode(&value).map_err(MutationError::Encode))
        .transpose()?;
    patch(&mut value).map_err(MutationError::Patch)?;
    domain.validate(&value).map_err(MutationError::Validation)?;
    let encoded = domain.encode(&value).map_err(MutationError::Encode)?;
    domain
        .validate_raw(domain.descriptor().schema_version(), &encoded)
        .map_err(MutationError::EncodedValueInvalid)?;
    if previous.as_ref() == Some(&encoded) {
        return Ok(MutationOutcome::Unchanged);
    }
    let document = SerializedDocument::new(
        domain.descriptor().id().clone(),
        domain.descriptor().schema_version(),
        encoded,
    );
    let bytes =
        serde_json::to_vec_pretty(&document).map_err(|error| MutationError::Serialization {
            detail: error.to_string(),
        })?;
    let durability =
        publish(&file, &bytes, options.durability).map_err(MutationError::Publication)?;

    Ok(MutationOutcome::Published(MutationReceipt {
        domain: domain.descriptor().id().clone(),
        path: file.full_path().to_path_buf(),
        schema_version: domain.descriptor().schema_version(),
        durability,
    }))
}

pub(crate) enum MutationOutcome {
    Unchanged,
    Published(MutationReceipt),
}

fn writable_file(location: DomainLocation) -> Result<ResolvedFile, MutationError> {
    match location {
        DomainLocation::File(file) if file.root_kind() == RootKind::Project => Err(
            MutationError::Refused(MutationRefusal::ProjectSharedRequiresExternalAuthority {
                path: file.full_path().to_path_buf(),
            }),
        ),
        DomainLocation::File(file) if file.access() == AccessMode::ReadOnly => {
            Err(MutationError::Refused(MutationRefusal::ReadOnly {
                path: file.full_path().to_path_buf(),
            }))
        }
        DomainLocation::File(file) => Ok(file),
        location => Err(MutationError::Refused(MutationRefusal::Unavailable {
            location,
        })),
    }
}
