//! The thing that sequences everything else.
//!
//! Cards 151, 152, 153 and 190 each built a piece: policy, sources,
//! quiescence, install, the protocol. Nothing held them together, and
//! `UpdateSnapshot` was constructed in exactly one place in the tree — a test.
//!
//! This is the controller the operator decision of 2026-08-12 made Longhorn's.
//! It observes; it does not perform. Every side effect leaves through a trait
//! the host implements: [`UpdateSource`] composes requests,
//! [`ArtifactFetch`](crate::ArtifactFetch) transfers,
//! [`QuiescenceProbe`](crate::QuiescenceProbe) reports what is in flight,
//! [`UpdateInstaller`] replaces the bundle.
//!
//! # Ordering, and where it diverges from Tauri's
//!
//! Check, prepare, apply. Since the 2026-09-22 amendment the middle of the
//! sequence is split: `prepare` fetches, verifies and retains an
//! identity-bound staged artifact, and `apply` gates and installs it. The gate
//! runs at apply rather than before the transfer, which is the one deliberate
//! difference in the sequence: downloading while the user has work in flight
//! is harmless, and gating before the transfer makes them wait for something
//! that could have happened in the background. Tauri gates nothing, because it
//! has no notion of Longhorn-owned work to gate on.
//!
//! The split is also what lets a host keep byte progress observable without
//! holding its own state lock: [`UpdateController::begin_prepare`] and
//! [`UpdateController::complete_prepare`] bracket the transfer, and the host
//! reports progress from inside it.
//!
//! # No clock
//!
//! Rollout staging and deferral both look time-shaped and neither needs a
//! time. The controller holds no `SystemTime` and takes none: a check happens
//! when something asks for one, and a deferral covers a version rather than a
//! duration.

use semver::Version;

use crate::{
    ArtifactFetch, ArtifactKey, BuildIdentity, ChannelManifest, CheckKind, Deferral, FetchError,
    InstallAuthorization, InstallFailure, InstallId, InstallProvenance, SourceError, SourceRequest,
    StagedArtifact, TargetTriple, UpdateApplyCommand, UpdateAvailability,
    UpdateAvailabilityProjection, UpdateCancelCommand, UpdateCheckCommand, UpdateDeferCommand,
    UpdateDeferralProjection, UpdateGate, UpdateInstaller, UpdateOutcomeProjection,
    UpdatePrepareCommand, UpdateProgressProjection, UpdateProtocolVersion, UpdateRejectionCode,
    UpdateSelectChannelCommand, UpdateSnapshot, UpdateSource, evaluate, verify_artifact,
};

/// A transfer `begin_prepare` started and `complete_prepare` has not seen finish.
///
/// It carries the authority lifetime it was started under. A channel switch
/// during the transfer replaces that lifetime, and `complete_prepare` refuses a
/// transfer whose bytes belong to a context that is gone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedTransfer {
    epoch: u64,
    version: Version,
    request: SourceRequest,
}

impl PreparedTransfer {
    /// Records a transfer a host is about to run on the controller's behalf.
    ///
    /// A host never composes one of these in production: `begin_prepare`
    /// returns it and the host hands the same value back. It stays public so
    /// an injected authority a test or an adapter implements can stand in for
    /// the controller without reassembling its offer logic.
    #[must_use]
    pub const fn new(authority_epoch: u64, version: Version, request: SourceRequest) -> Self {
        Self {
            epoch: authority_epoch,
            version,
            request,
        }
    }

    /// The authority lifetime the transfer was started under.
    #[must_use]
    pub const fn authority_epoch(&self) -> u64 {
        self.epoch
    }

    /// The version being transferred.
    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    /// The request the host transfers.
    #[must_use]
    pub const fn request(&self) -> &SourceRequest {
        &self.request
    }
}

/// What `begin_prepare` produced: a transfer to run, or a refusal.
///
/// A refusal is an ordinary controller outcome -- no offer, or a stale caller
/// -- not a host error, so it travels as one.
#[derive(Clone, Debug)]
pub enum UpdatePrepareStart {
    /// The host transfers this request and hands the result back.
    Transfer(PreparedTransfer),
    /// The controller refused before any transfer started.
    Refused(UpdateOutcomeProjection),
}

/// Holds update state and answers the protocol's four commands.
///
/// Borrows its ports rather than owning them, as [`UpdateGate`] borrows its
/// probes. A host wires them once and keeps them.
pub struct UpdateController<'port> {
    build: BuildIdentity,
    target: TargetTriple,
    install: InstallId,
    provenance: InstallProvenance,
    key: ArtifactKey,
    source: &'port dyn UpdateSource,
    fetch: &'port dyn ArtifactFetch,
    /// Optimistic concurrency for commands, not a replay ledger.
    ///
    /// Advances when the controller's authority context is replaced —
    /// today that is `select_channel`. A command carries the epoch from the
    /// snapshot it was issued against, so one issued before a channel switch
    /// is refused as stale rather than applied to state it never saw. A
    /// process restart resets the epoch to 1, which is honest: commands do
    /// not survive a restart, and re-executing a check is idempotent.
    authority_epoch: u64,
    manifest: Option<ChannelManifest>,
    availability: UpdateAvailability,
    deferral: Option<Deferral>,
    progress: UpdateProgressProjection,
    /// The one retained verified artifact, when one is staged.
    ///
    /// Exactly one per controller. It is the amendment's whole reason to
    /// exist: a surface can hold an update through "Later" without the
    /// controller re-fetching or re-verifying it.
    staged: Option<StagedArtifact>,
    /// Whether a split prepare's transfer is in flight.
    ///
    /// `begin_prepare` and `complete_prepare` are two calls with a transfer
    /// between them, so something has to remember that the interval is open.
    /// The convenience `prepare` closes it within one call.
    preparing: bool,
    /// A cancel that arrived while a transfer was in flight.
    ///
    /// The transfer cannot be stopped once the host is running it, so the
    /// cancel takes effect where it can: `complete_prepare` discards what the
    /// transfer delivers rather than retaining it.
    discard_transfer: bool,
}

impl<'port> UpdateController<'port> {
    /// Records a controller for one install.
    ///
    /// `provenance` is supplied rather than detected: detection reads the
    /// filesystem, which this crate does not do. `classify_install` gives the
    /// host the answer to pass in.
    pub fn new(
        build: BuildIdentity,
        target: TargetTriple,
        install: InstallId,
        provenance: InstallProvenance,
        key: ArtifactKey,
        source: &'port dyn UpdateSource,
        fetch: &'port dyn ArtifactFetch,
    ) -> Self {
        Self {
            build,
            target,
            install,
            provenance,
            key,
            source,
            fetch,
            authority_epoch: 1,
            manifest: None,
            // Nothing has been checked, which is not the same as being up to
            // date. `UpToDate` is what an actual check found; this is the
            // honest state before one, and the two differ to a surface that
            // says "last checked".
            availability: UpdateAvailability::UpToDate,
            deferral: None,
            progress: UpdateProgressProjection::Idle,
            staged: None,
            preparing: false,
            discard_transfer: false,
        }
    }

    /// The live authority lifetime.
    #[must_use]
    pub const fn authority_epoch(&self) -> u64 {
        self.authority_epoch
    }

    /// The state as a client reads it.
    #[must_use]
    pub fn snapshot(&self) -> UpdateSnapshot {
        UpdateSnapshot {
            protocol_version: UpdateProtocolVersion::CURRENT,
            authority_epoch: self.authority_epoch,
            channel: self.build.channel,
            installed_version: self.build.version.to_string(),
            availability: UpdateAvailabilityProjection::from_availability(&self.availability),
            deferral: self
                .deferral
                .as_ref()
                .map(UpdateDeferralProjection::from_deferral),
            staged: self.staged.as_ref().map(StagedArtifact::projection),
            progress: self.progress.clone(),
        }
    }

    /// The request that retrieves the selected channel's manifest.
    ///
    /// The host performs it and hands the parsed manifest to [`Self::check`].
    /// Deserialising is the host's because it is where the transport already
    /// is, and a JSON parser in the policy crate would be one more thing to
    /// keep pure for no gain.
    pub fn manifest_request(&self) -> Result<crate::SourceRequest, SourceError> {
        self.source.manifest_request(self.build.channel)
    }

    /// Records what a check found.
    pub fn check(
        &mut self,
        command: &UpdateCheckCommand,
        manifest: &ChannelManifest,
        kind: CheckKind,
    ) -> UpdateOutcomeProjection {
        if let Some(stale) = self.refuse_if_stale(command.authority_epoch) {
            return stale;
        }

        // The endpoint for the selected channel served a manifest claiming a
        // different one. Evaluating it would let a mislabel silently restage
        // a rollout, so the manifest is refused and nothing is stored.
        if manifest.channel != self.build.channel {
            return self.reject(UpdateRejectionCode::ChannelMismatch);
        }

        self.availability = evaluate(&self.build, manifest, &self.install, kind, self.provenance);
        self.manifest = Some(manifest.clone());
        self.commit()
    }

    /// Follows a different channel from now on.
    ///
    /// The recorded availability is dropped rather than kept. It was an answer
    /// about the old channel, and a surface showing it beside the new one
    /// would be showing a stale offer as a current one.
    pub fn select_channel(
        &mut self,
        command: &UpdateSelectChannelCommand,
    ) -> UpdateOutcomeProjection {
        if let Some(stale) = self.refuse_if_stale(command.authority_epoch) {
            return stale;
        }

        self.build.channel = command.channel;
        // The authority context was replaced: commands issued against the
        // pre-switch snapshot must refuse rather than act on state they
        // never saw. This is what makes `StaleAuthority` fireable.
        self.authority_epoch += 1;
        self.manifest = None;
        self.availability = UpdateAvailability::UpToDate;
        self.deferral = None;
        // The staged artifact belongs to the channel that was followed. It is
        // dropped rather than carried into the new one: the new channel may
        // publish a different version, and a staged file from another
        // channel's release is an update the operator never selected.
        self.staged = None;
        self.progress = UpdateProgressProjection::Idle;
        self.commit()
    }

    /// Declines a version for now.
    pub fn defer(&mut self, command: &UpdateDeferCommand) -> UpdateOutcomeProjection {
        if let Some(stale) = self.refuse_if_stale(command.authority_epoch) {
            return stale;
        }

        let Ok(version) = command.version.parse::<Version>() else {
            return self.reject(UpdateRejectionCode::NoOffer);
        };
        self.deferral = Some(Deferral::new(version, command.cause.clone()));
        self.progress = UpdateProgressProjection::Idle;
        self.commit()
    }

    /// Starts a staged prepare: validates the offer and composes the transfer.
    ///
    /// Split from [`Self::complete_prepare`] so a host whose controller sits
    /// behind a shared-state lock can release that lock across the transfer.
    /// A host with no such lock calls [`Self::prepare`] instead, which runs
    /// both halves in one call.
    pub fn begin_prepare(&mut self, command: &UpdatePrepareCommand) -> UpdatePrepareStart {
        if let Some(stale) = self.refuse_if_stale(command.authority_epoch) {
            return UpdatePrepareStart::Refused(stale);
        }

        // An externally managed install never reaches an `Offer` -- `evaluate`
        // returns `ManagedElsewhere` before any offer path -- so this refuses
        // before the transfer rather than after eighty megabytes.
        let Some((version, artifact)) = self.offered_artifact(&command.version) else {
            return UpdatePrepareStart::Refused(self.reject(UpdateRejectionCode::NoOffer));
        };

        let request = match self.source.artifact_request(&artifact) {
            Ok(request) => request,
            Err(_) => {
                return UpdatePrepareStart::Refused(self.reject(UpdateRejectionCode::Unavailable));
            }
        };

        // A new prepare supersedes whatever was staged: exactly one staged
        // artifact is retained per controller.
        self.staged = None;
        self.preparing = true;
        self.discard_transfer = false;
        self.progress = UpdateProgressProjection::Downloading {
            received: 0,
            expected: None,
            fraction: None,
        };
        UpdatePrepareStart::Transfer(PreparedTransfer {
            epoch: self.authority_epoch,
            version,
            request,
        })
    }

    /// Verifies and retains what a started transfer delivered.
    pub fn complete_prepare(
        &mut self,
        transfer: &PreparedTransfer,
        delivered: Result<Vec<u8>, FetchError>,
    ) -> UpdateOutcomeProjection {
        self.preparing = false;

        // A channel switch during the transfer replaced the authority
        // lifetime, so the bytes belong to a context that is gone.
        if transfer.epoch != self.authority_epoch {
            self.progress = UpdateProgressProjection::Idle;
            return self.reject(UpdateRejectionCode::StaleAuthority);
        }
        // The operator cancelled while the transfer was running. The transfer
        // could not be stopped; what it delivered will not be retained.
        if self.discard_transfer {
            self.discard_transfer = false;
            self.progress = UpdateProgressProjection::Idle;
            return self.commit();
        }

        let bytes = match delivered {
            Ok(bytes) => bytes,
            Err(error) => {
                self.progress = UpdateProgressProjection::Idle;
                return self.reject(match error {
                    FetchError::Interrupted { .. } => UpdateRejectionCode::Unreachable,
                    FetchError::Unavailable { .. } => UpdateRejectionCode::Unavailable,
                });
            }
        };
        // The limit is passed to the host, and also checked here. A host that
        // overruns it has already spent the memory, so this cannot undo the
        // cost -- but it stops the oversized buffer going any further, and it
        // makes the bound a property of the controller rather than a request
        // every implementor is trusted to have honoured.
        //
        // Deliberately unproven: reaching it from a test means a fake host
        // allocating two gigabytes, and a seam to shrink the limit would be
        // larger than the check. The extraction quotas, which a hostile
        // archive can reach cheaply, are proved instead.
        if bytes.len() as u64 > crate::MAX_ARTIFACT_BYTES {
            self.progress = UpdateProgressProjection::Idle;
            return self.reject(UpdateRejectionCode::Unavailable);
        }

        // The offer must still stand. A check between begin and complete can
        // withdraw it, and bytes for a version the controller no longer
        // offers are not retained.
        let Some((version, artifact)) = self.offered_artifact(&transfer.version.to_string()) else {
            self.progress = UpdateProgressProjection::Idle;
            return self.reject(UpdateRejectionCode::NoOffer);
        };

        self.progress = UpdateProgressProjection::Verifying;
        let verified = match verify_artifact(&self.key, &version, bytes, &artifact.signature) {
            Ok(verified) => verified,
            Err(_) => {
                // A verification failure is discarded, never retained. This
                // is the trust boundary the staging reversal did not move.
                self.progress = UpdateProgressProjection::Idle;
                return self.reject(UpdateRejectionCode::SignatureRejected);
            }
        };

        self.staged = Some(StagedArtifact::new(
            version.clone(),
            self.build.channel,
            verified,
        ));
        self.progress = UpdateProgressProjection::ReadyToInstall {
            version: version.to_string(),
        };
        self.commit()
    }

    /// Fetches, verifies, and retains, reporting progress as bytes arrive.
    ///
    /// The one-call form of [`Self::begin_prepare`] plus
    /// [`Self::complete_prepare`]. `report` runs inside the transfer, so a
    /// surface renders byte progress rather than waiting for the call to
    /// return.
    pub fn prepare(
        &mut self,
        command: &UpdatePrepareCommand,
        report: &mut dyn FnMut(&UpdateProgressProjection),
    ) -> UpdateOutcomeProjection {
        let transfer = match self.begin_prepare(command) {
            UpdatePrepareStart::Transfer(transfer) => transfer,
            UpdatePrepareStart::Refused(outcome) => return outcome,
        };
        let fetch = self.fetch;
        let delivered = fetch.fetch(
            &transfer.request,
            crate::MAX_ARTIFACT_BYTES,
            &mut |progress| report(&UpdateProgressProjection::from(progress)),
        );
        self.complete_prepare(&transfer, delivered)
    }

    /// Applies the retained staged artifact.
    ///
    /// The gate runs here rather than at prepare time: downloading while the
    /// user has work in flight is harmless, and a refusal is a deferral that
    /// leaves the artifact staged for a later apply.
    ///
    /// Since the 2026-09-22 amendment the gate returns a held exclusive
    /// admission lease, and this method keeps it alive across
    /// [`UpdateInstaller::apply`] — released only once replacement has
    /// returned. An install with an unacquired lease is unreachable from
    /// here: the only way past the match below is a `Held`.
    pub fn apply<I: UpdateInstaller>(
        &mut self,
        command: &UpdateApplyCommand,
        gate: &UpdateGate<'_>,
        installer: &I,
    ) -> UpdateOutcomeProjection {
        if let Some(stale) = self.refuse_if_stale(command.authority_epoch) {
            return stale;
        }

        let Some(staged) = self.staged.take() else {
            return self.reject(UpdateRejectionCode::NoOffer);
        };
        if staged.version().to_string() != command.version {
            self.staged = Some(staged);
            return self.reject(UpdateRejectionCode::NoOffer);
        }
        let version = staged.version().clone();

        // Authorization is a held answer now: `Held` owns the exclusive
        // admission lease, and this binding keeps it alive until the
        // replacement has returned.
        let lease = match gate.authorize(&version) {
            InstallAuthorization::Held(lease) => lease,
            InstallAuthorization::Deferred(deferral) => {
                // Not a failure, and not a cancellation: a refused install
                // carries its reason and the artifact stays staged for the
                // next attempt.
                self.deferral = Some(deferral);
                self.staged = Some(staged);
                return self.commit();
            }
        };

        self.progress = UpdateProgressProjection::Installing {
            version: version.to_string(),
        };
        let applied = installer.apply(staged.verified());
        // The barrier is released only once replacement has returned. Dropping
        // it here rather than at scope end makes the ordering explicit: work
        // that started mid-swap is the failure the lease exists to prevent.
        drop(lease);
        match applied {
            Ok(_) => {
                self.deferral = None;
                self.availability = UpdateAvailability::UpToDate;
                self.build.version = version;
                self.progress = UpdateProgressProjection::Idle;
                self.commit()
            }
            Err(failure) => {
                // Replacement failed, not verification. The bytes are still
                // verified and still staged, so a retry does not re-download.
                self.progress = UpdateProgressProjection::ReadyToInstall {
                    version: version.to_string(),
                };
                self.staged = Some(staged);
                self.reject(match failure {
                    InstallFailure::SignatureRejected => UpdateRejectionCode::SignatureRejected,
                    InstallFailure::NotWritable { .. } => UpdateRejectionCode::NotWritable,
                    InstallFailure::MalformedArtifact { .. } | InstallFailure::Failed { .. } => {
                        UpdateRejectionCode::InstallFailed
                    }
                })
            }
        }
    }

    /// Discards the retained staged artifact.
    ///
    /// Returns to `Idle`. A transfer already running cannot be stopped, so a
    /// cancel that arrives mid-transfer takes effect at completion instead:
    /// what it delivers is discarded rather than staged.
    pub fn cancel(&mut self, command: &UpdateCancelCommand) -> UpdateOutcomeProjection {
        if let Some(stale) = self.refuse_if_stale(command.authority_epoch) {
            return stale;
        }
        if self.preparing {
            self.discard_transfer = true;
        }
        self.staged = None;
        self.progress = UpdateProgressProjection::Idle;
        self.commit()
    }

    /// The artifact for a requested version, when one is actually offered.
    fn offered_artifact(&self, requested: &str) -> Option<(Version, crate::Artifact)> {
        let UpdateAvailability::Offer(offer) = &self.availability else {
            return None;
        };
        if offer.version.to_string() != requested {
            return None;
        }
        let artifact = self.manifest.as_ref()?.artifacts.get(&self.target)?;
        Some((offer.version.clone(), artifact.clone()))
    }

    fn refuse_if_stale(&mut self, observed: u64) -> Option<UpdateOutcomeProjection> {
        (observed != self.authority_epoch).then(|| self.reject(UpdateRejectionCode::StaleAuthority))
    }

    fn commit(&mut self) -> UpdateOutcomeProjection {
        UpdateOutcomeProjection::Committed {
            snapshot: self.snapshot(),
        }
    }

    fn reject(&self, code: UpdateRejectionCode) -> UpdateOutcomeProjection {
        UpdateOutcomeProjection::Rejected {
            code,
            snapshot: self.snapshot(),
        }
    }
}
