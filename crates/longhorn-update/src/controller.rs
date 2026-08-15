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
//! Check, fetch, verify, gate, install. The gate sits between verify and
//! install rather than before fetch, which is the one deliberate difference in
//! the sequence: downloading while the user has work in flight is harmless,
//! and gating before the transfer makes them wait for something that could
//! have happened in the background. Tauri gates nothing, because it has no
//! notion of Longhorn-owned work to gate on.
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
    FetchProgress, InstallAuthorization, InstallFailure, InstallId, InstallProvenance, SourceError,
    TargetTriple, UpdateAvailability, UpdateAvailabilityProjection, UpdateCheckCommand,
    UpdateDeferCommand, UpdateDeferralProjection, UpdateGate, UpdateInstallCommand,
    UpdateInstaller, UpdateOutcomeProjection, UpdateProgressProjection, UpdateProtocolVersion,
    UpdateRejectionCode, UpdateSelectChannelCommand, UpdateSnapshot, UpdateSource, evaluate,
    verify_artifact,
};

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

    /// Fetches, verifies, gates and installs.
    ///
    /// The whole sequence in one call because it is one operator action. A
    /// caller that wanted the steps separately would have to hold a
    /// half-finished transfer between them, and there is deliberately no type
    /// for that — see [`ArtifactFetch`](crate::ArtifactFetch).
    pub fn install<I: UpdateInstaller>(
        &mut self,
        command: &UpdateInstallCommand,
        gate: &UpdateGate<'_>,
        installer: &I,
    ) -> UpdateOutcomeProjection {
        if let Some(stale) = self.refuse_if_stale(command.authority_epoch) {
            return stale;
        }

        // An externally managed install never reaches an `Offer` -- `evaluate`
        // returns `ManagedElsewhere` before any offer path -- so this refuses
        // before the transfer rather than after eighty megabytes.
        let Some((version, artifact)) = self.offered_artifact(&command.version) else {
            return self.reject(UpdateRejectionCode::NoOffer);
        };

        let request = match self.source.artifact_request(&artifact) {
            Ok(request) => request,
            Err(_) => return self.reject(UpdateRejectionCode::Unavailable),
        };

        self.progress = UpdateProgressProjection::Downloading { fraction: None };
        let mut observed: Option<FetchProgress> = None;
        let bytes = match self
            .fetch
            .fetch(&request, crate::MAX_ARTIFACT_BYTES, &mut |progress| {
                observed = Some(progress);
            }) {
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
        // The last report the host made, not a count of what arrived. A host
        // that reports nothing leaves the fraction absent, which is the same
        // answer as a source with no content length and is honest for the same
        // reason.
        self.progress = UpdateProgressProjection::Downloading {
            fraction: observed.and_then(FetchProgress::fraction),
        };

        self.progress = UpdateProgressProjection::Verifying;
        let verified = match verify_artifact(&self.key, &version, bytes, &artifact.signature) {
            Ok(verified) => verified,
            Err(_) => {
                self.progress = UpdateProgressProjection::Idle;
                return self.reject(UpdateRejectionCode::SignatureRejected);
            }
        };

        self.progress = UpdateProgressProjection::ReadyToInstall {
            version: version.to_string(),
        };
        if let InstallAuthorization::Deferred(deferral) = gate.authorize(&version) {
            // Not a failure. A refused install carries its reason, and the
            // deferral is the reason.
            self.deferral = Some(deferral);
            self.progress = UpdateProgressProjection::Idle;
            return self.commit();
        }

        self.progress = UpdateProgressProjection::Installing {
            version: version.to_string(),
        };
        match installer.apply(&verified) {
            Ok(_) => {
                self.deferral = None;
                self.availability = UpdateAvailability::UpToDate;
                self.build.version = version;
                self.progress = UpdateProgressProjection::Idle;
                self.commit()
            }
            Err(failure) => {
                self.progress = UpdateProgressProjection::Idle;
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
