# 018 Application Update And Release Channels

Status: active compiled boundary
Owner: Tom
Updated: 2026-08-07
Architecture: `../architecture/system-architecture.md`
Research: `../research/translation-memos/019-application-update-and-release-channels.md`

## Boundary

Longhorn owns update *policy* — channel resolution, version comparison,
rollout eligibility, deferral, and restart readiness. Longhorn does not own
artifact hosting, signing, or the installation mechanism. Consuming
applications own their release cadence, their signing identity, and where
their artifacts live.

## Verification And Trust

- Artifact signature verification belongs to the Tauri updater plugin.
  Longhorn never implements, wraps, or bypasses it.
- Because every artifact is verified against a key compiled into the
  application, the artifact host is untrusted infrastructure. No adapter may
  claim a security property on the basis of its transport.
- An update whose signature does not verify is not an error to be reported
  and retried. It is discarded.

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

## Diagnostics

- Channel and build identity are stamped into the diagnostics seam so a
  report attributes to the line that produced it.

## Non-goals

- Artifact hosting, signing key custody, and notarization remain
  consumer-owned.
- Delta updates, rollback, and server-side rollout orchestration are out of
  scope for the compiled boundary.
- Longhorn publishes no update server.
