# 018 Application Update And Release Channels

Status: active compiled boundary
Owner: Tom
Updated: 2026-08-09
Architecture: `../architecture/system-architecture.md`
Research: `../research/translation-memos/019-application-update-and-release-channels.md`

## Boundary

Longhorn owns update *policy* — channel resolution, version comparison,
rollout eligibility, deferral, and restart readiness. Longhorn does not own
artifact hosting or signing. Consuming applications own their release
cadence, their signing identity, and where their artifacts live.

**Amended 2026-08-09 — execution is host-independent.** Longhorn owns update
execution on every host. One implementation, `longhorn-update-install`, serves
Tauri and GPUI alike.

This supersedes the 2026-08-08 amendment, which made execution host-dependent
— Tauri hosts using the updater plugin, non-Tauri hosts using Longhorn's
native implementation, both satisfying one conformance suite. That decision
assumed the plugin could verify an artifact Longhorn hands it. Card 162
established, from `tauri-plugin-updater` 2.10.1, that it cannot:

- `verify_signature` is called in exactly one place, at the end of
  `Update::download`.
- `Update::install(bytes)` reaches the platform installer with no
  verification of any kind.
- `Update`'s fields are private and only a network `check()` constructs one,
  so no adapter can wrap bytes it already holds in one.

So an adapter must either surrender the artifact to the plugin's own
downloader, or hand the plugin unverified bytes. The second violates this
contract absolutely. The first is a different contract. **There is no
implementation of the shared suite over the plugin**, which makes the
"two implementations, one suite" claim unsatisfiable rather than merely
unproven.

The deciding argument is the direction of the guarantee: the host with no
plugin must work solidly, and an implementation that satisfies GPUI
necessarily satisfies Tauri, since nothing in it is host-specific. Building
for the weaker host and letting the stronger one inherit is the only ordering
that leaves no host under-served.

**Tauri's updater remains the specification, not the mechanism.** Its macOS
install path defines the artifact shape — a gzip tar whose single top-level
entry is the application — and Longhorn matches it exactly, so one signed
release serves both hosts. Longhorn diverges only where the plugin's approach
is unsafe to copy: no shell interpolation, classified failures, and bounded
extraction. See `longhorn-update-install`.

Authorization is unchanged and remains host-agnostic: `UpdateGate::authorize`
answers whether an install may proceed, whoever performs it.

**Windows is the open edge.** Longhorn's installer covers macOS bundle
replacement. NSIS and MSI are unimplemented, and the plugin is the obvious
donor specification when they are needed — as a specification, on the same
terms as the macOS path.

## Verification And Trust

- Artifact signature verification belongs to whichever component installs,
  and on every host that is Longhorn. One verifier, one key, one signed
  release, both hosts.
- An installer that does not verify is not an installer. There is no
  configuration, host, or build profile under which an unverified artifact
  may be applied.
- Because every artifact is verified against a key compiled into the
  application, the artifact host is untrusted infrastructure. No adapter may
  claim a security property on the basis of its transport.
- Untrusted infrastructure bounds resources too. A signature proves origin,
  not intent, and size is a resource dimension the signature does not cover.
  The transfer stops at `MAX_ARTIFACT_BYTES` (2 GiB): the controller passes
  the limit to the host so a conforming fetch stops early rather than
  buffering, and checks the delivered length itself, so the bound is a
  property of the controller rather than a request each implementor is
  trusted to have honoured.
- Extraction stops at two quotas, not one: 4 GiB of declared entry size, and
  250,000 entries. Both are load-bearing. Directories, links and empty files
  declare no bytes at all, so an archive built from millions of them passes a
  byte quota untouched while still exhausting inodes and directory space —
  bytes are not the only finite resource an extraction spends. The quotas are
  injectable internally so both are proved against a small archive, rather
  than by building the hostile artifact they exist to refuse.
- The signature may bind the release version through minisign's trusted
  comment (`version:<semver>`), which the global signature covers. The
  verifier enforces the binding when present and refuses a mismatch as a
  signature failure — that is the downgrade defence. Signatures without the
  comment (Tauri's signing emits a timestamp) still verify; making the
  comment mandatory is a distribution-format decision, deferred because it
  changes what the consumer's signing step must produce.
- Privileged replacement receives the verified artifact, never a staged
  tree. The signature covers the archive bytes, and a tree staged where the
  user can write is mutable between unpack and the privileged move. An
  escalation implementor extracts the artifact itself, into protected
  staging, with the same bounded extraction (`extract_bundle`).
- An update whose signature does not verify is not an error to be reported
  and retried. It is discarded.

## Installation Provenance

- An application a package manager installed is **never offered a
  self-update**. Replacing it leaves the manager describing a version that is
  no longer on disk, and the next upgrade through that manager reverts or
  fails.
- Provenance is checked before an update is offered, not when an install is
  attempted. Writability is the wrong signal: a Homebrew cask lands in
  `/Applications`, which is group-writable by admin users, so a permission
  check passes and the desync happens silently.
- An externally managed installation is told **where** the update is, never
  that there is none. `ManagedElsewhere` is a distinct availability from
  `UpToDate`, because reporting no update when one exists is false.
- This overrides the mandatory-version floor. A security release is still not
  withheld — the user is told where to get it — but urgency does not make it
  safe to desync a package database.
- Detection that cannot reach a confident answer returns `Undetermined`, and
  `Undetermined` updates normally. A false "externally managed" blocks a
  legitimate update; a false "self-managed" corrupts a package database.
  Neither is acceptable, and only one of them is the status quo.
- Windows package managers are **unproven**. winget, Chocolatey and MSIX
  identity are detectable in principle and are not detected today.

## Update Sources

- A source adapter supplies a channel manifest and an artifact request. It
  never downloads, verifies, or installs.
- An artifact request carries a URL and optional headers, because
  authenticated hosts cannot express credentials in a URL alone.
- Longhorn ships default adapters for static JSON, public GitHub releases, a
  secondary public releases repository, and S3/R2 with optional presigning.
  Consumers may implement their own and inherit all policy below.
- Adapters are infallible in policy terms: a source that cannot be reached
  yields no update, never a degraded or unverified one.

## Channels

- Channels are `production`, `beta`, and `nightly`, resolved at runtime from
  persisted settings, not baked in at build time.
- All channels ship under one bundle identity. Side-by-side installation is
  not a Longhorn-supported shape.
- Version ordering is semver, including prerelease ordering. Channel rejoin
  is a consequence of that ordering and is not special-cased.
- An install ahead of its selected channel is a supported state and must be
  surfaced as such, never as "no update available".
- Longhorn does not downgrade by version comparison. Moving an install
  backwards is an explicit, separately authorized action.

## Rollout

- Rollout eligibility is evaluated on the client, so that static hosting
  remains sufficient.
- Eligibility is a deterministic function of a persisted random install
  identifier and a manifest-supplied seed. It never varies between checks
  for the same install and release.
- The install identifier is random per installation. It is never derived
  from hardware, user identity, or any other stable external fact.
- A `minimum_version` floor overrides rollout entirely: installs below it
  are always offered the update.
- A user-initiated check bypasses rollout.
- Rollout applies to `production`. `beta` and `nightly` are offered in full.

## Restart Readiness

- No update installs while Longhorn-owned work is in flight. The updater
  obtains a quiescence receipt from the lifecycle coordinator before
  handing over.
- Quiescence covers pending flushes, uncommitted transfer sessions, and
  in-flight async operations.
- A refused restart is deferred, not cancelled, and the deferral is
  surfaced with its reason.
- Restart readiness is a Longhorn responsibility. A consuming application is
  never asked to determine it.

## Store Compatibility Across Channels

- Every persistent store records the schema version that wrote it.
- A reader encountering a newer schema version refuses to load and reports
  the mismatch. It never parses partially, and never writes back a store it
  could not fully read.
- This applies to configuration, settings, history, and history-tree
  equally, and to backup archives.
- Each store keeps its own version vocabulary. One shared classification
  answers whether a refusal was a future-schema refusal, so a client surface
  can explain a channel rejoin without matching per-store error shapes.

## Reporting

- An install that reached disk but did not relaunch is **not** a failed
  update. The correct message asks the user to reopen the application;
  reporting failure is false and invites retrying an update they already
  have. This is not hypothetical — macOS relaunch is known to fail
  (tauri#11392).
- A crash between the swap's two renames is recovered, not reported: the
  next apply restores the displaced `*.longhorn-previous` install before
  attempting anything new. Rollback as a user-facing feature stays a
  non-goal; never bricking the install is a mechanism property.
- A refused install is deferred, never cancelled, and carries its reason.

## Diagnostics

- Channel and build identity are stamped into the diagnostics seam so a
  report attributes to the line that produced it.

## Non-goals

- Artifact hosting, signing key custody, and notarization remain
  consumer-owned.
- Delta updates, rollback, and server-side rollout orchestration are out of
  scope for the compiled boundary.
- Longhorn publishes no update server.
