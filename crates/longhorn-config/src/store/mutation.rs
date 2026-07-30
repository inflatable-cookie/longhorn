use crate::{AccessMode, ConfigDomain, DomainIssue, DomainLocation, ResolvedFile, RootKind};

use super::{
    CheckedMutationContext, CheckedMutationError, CheckedMutationOutcome, ConfigStore, LoadOutcome,
    LoadedOrigin, MutationError, MutationOptions, MutationReceipt, MutationRefusal,
    UnavailableState, document::SerializedDocument, load::load_file, publication::publish,
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
    match mutate_inner(
        store,
        domain,
        options,
        |context| patch(context.value_mut()),
        false,
    ) {
        Ok(outcome) => Ok(outcome
            .publication
            .expect("immediate mutation always publishes")),
        Err(CheckedMutationError::Check(issue)) => Err(MutationError::Patch(issue)),
        Err(CheckedMutationError::Mutation(error)) => Err(error),
    }
}

pub(super) fn mutate_checked<D, R, E, F>(
    store: &ConfigStore,
    domain: &D,
    options: MutationOptions,
    check_and_patch: F,
) -> Result<CheckedMutationOutcome<R, D::Value>, CheckedMutationError<E>>
where
    D: ConfigDomain,
    F: FnOnce(&mut CheckedMutationContext<'_, D::Value>) -> Result<R, E>,
{
    let outcome = mutate_inner(store, domain, options, check_and_patch, true)?;
    Ok(CheckedMutationOutcome::new(
        outcome.checked,
        outcome.value,
        outcome.publication,
    ))
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
    match mutate_inner(
        store,
        domain,
        options,
        |context| patch(context.value_mut()),
        true,
    ) {
        Ok(outcome) => Ok(match outcome.publication {
            Some(receipt) => MutationOutcome::Published(receipt),
            None => MutationOutcome::Unchanged,
        }),
        Err(CheckedMutationError::Check(issue)) => Err(MutationError::Patch(issue)),
        Err(CheckedMutationError::Mutation(error)) => Err(error),
    }
}

fn mutate_inner<D, R, E, F>(
    store: &ConfigStore,
    domain: &D,
    options: MutationOptions,
    check_and_patch: F,
    skip_unchanged: bool,
) -> Result<InnerMutationOutcome<R, D::Value>, CheckedMutationError<E>>
where
    D: ConfigDomain,
    F: FnOnce(&mut CheckedMutationContext<'_, D::Value>) -> Result<R, E>,
{
    store
        .ensure_registered(domain)
        .map_err(MutationError::Store)
        .map_err(CheckedMutationError::Mutation)?;
    let location = store.roots.resolve(domain.descriptor());
    let file = writable_file(location).map_err(CheckedMutationError::Mutation)?;
    let guard = store
        .coordinator
        .acquire(options.lock_timeout)
        .map_err(MutationError::Coordination)
        .map_err(CheckedMutationError::Mutation)?;
    crate::backup::restore::recover_guarded(store, &guard)
        .map_err(MutationError::RestoreRecovery)
        .map_err(CheckedMutationError::Mutation)?;
    let loaded = load_file(domain, &file);

    let loaded = match loaded {
        LoadOutcome::Ready(loaded) => match loaded.origin {
            LoadedOrigin::Default | LoadedOrigin::File => loaded,
            LoadedOrigin::MigratedInMemory { from, to } => {
                return Err(CheckedMutationError::Mutation(MutationError::Refused(
                    MutationRefusal::MigrationBackupRequired { from, to },
                )));
            }
        },
        LoadOutcome::Recovery(recovery) => {
            return Err(CheckedMutationError::Mutation(MutationError::Refused(
                MutationRefusal::Recovery(recovery),
            )));
        }
        LoadOutcome::Unavailable(UnavailableState::Authority { location }) => {
            return Err(CheckedMutationError::Mutation(MutationError::Refused(
                MutationRefusal::Unavailable { location },
            )));
        }
        LoadOutcome::Unavailable(
            UnavailableState::RestoreActive | UnavailableState::RestoreRecoveryRequired,
        ) => {
            unreachable!("mutation loads only after coordinated recovery");
        }
    };

    let mut value = loaded.value;
    let previous = skip_unchanged
        .then(|| {
            domain
                .encode(&value)
                .map_err(MutationError::Encode)
                .map_err(CheckedMutationError::Mutation)
        })
        .transpose()?;
    let checked = check_and_patch(&mut CheckedMutationContext {
        value: &mut value,
        schema_version: loaded.schema_version,
        origin: loaded.origin,
        diagnostics: &loaded.diagnostics,
        source: loaded.source.as_ref(),
    })
    .map_err(CheckedMutationError::Check)?;
    domain
        .validate(&value)
        .map_err(MutationError::Validation)
        .map_err(CheckedMutationError::Mutation)?;
    let encoded = domain
        .encode(&value)
        .map_err(MutationError::Encode)
        .map_err(CheckedMutationError::Mutation)?;
    domain
        .validate_raw(domain.descriptor().schema_version(), &encoded)
        .map_err(MutationError::EncodedValueInvalid)
        .map_err(CheckedMutationError::Mutation)?;
    if previous.as_ref() == Some(&encoded) {
        return Ok(InnerMutationOutcome {
            checked,
            value,
            publication: None,
        });
    }
    let document = SerializedDocument::new(
        domain.descriptor().id().clone(),
        domain.descriptor().schema_version(),
        encoded,
    );
    let bytes = serde_json::to_vec_pretty(&document)
        .map_err(|error| MutationError::Serialization {
            detail: error.to_string(),
        })
        .map_err(CheckedMutationError::Mutation)?;
    let durability = publish(&file, &bytes, options.durability)
        .map_err(MutationError::Publication)
        .map_err(CheckedMutationError::Mutation)?;

    Ok(InnerMutationOutcome {
        checked,
        value,
        publication: Some(MutationReceipt {
            domain: domain.descriptor().id().clone(),
            path: file.full_path().to_path_buf(),
            schema_version: domain.descriptor().schema_version(),
            durability,
        }),
    })
}

pub(crate) enum MutationOutcome {
    Unchanged,
    Published(MutationReceipt),
}

struct InnerMutationOutcome<R, T> {
    checked: R,
    value: T,
    publication: Option<MutationReceipt>,
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
