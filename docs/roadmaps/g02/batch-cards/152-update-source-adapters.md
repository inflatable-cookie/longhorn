# 152 Update Source Adapters

Status: complete
Owner: Tom
Roadmap: g02.009 batch 2
Governing refs: contract 018; research memo 019
Depends on: Card 151
Auto-start next card: no
Completed: 2026-08-07

## Objective

Define the `UpdateSource` trait and ship four default adapters, so a
consuming application can host artifacts wherever it likes and inherit all
update policy unchanged.

## Scope

- the `UpdateSource` trait: fetch a channel manifest, produce an artifact
  request
- default adapters: static JSON, public GitHub releases, secondary public
  releases repository, S3/R2 with optional presigning
- documentation of the private-GitHub case and why it needs a proxy

## Steps

1. Define the trait as two operations. Fetching a manifest and describing
   how to retrieve an artifact are separate concerns: an artifact request
   carries a URL *and optional headers*, because authenticated hosts cannot
   express credentials in a URL alone. Private GitHub asset download needs
   an asset-ID call plus `Accept: application/octet-stream`; presigned S3
   and R2 URLs need no headers at all. Both must fit.
2. Implement the static JSON adapter first — it is the reference shape and
   the one every other adapter degrades to.
3. Implement public GitHub releases, then the secondary-releases-repository
   variant as the same code against a different repository coordinate.
4. Implement S3/R2, with presigning optional so a public bucket needs no
   credentials.
5. Establish that a source which cannot be reached yields no update, never a
   degraded or unverified one. An adapter has no authority to weaken policy.
6. Document the private-GitHub position: asset URLs cannot be authenticated
   directly and the workable route is a proxy. Say so rather than shipping
   an adapter that half-works.
7. Tests: each adapter against a recorded manifest fixture; header
   propagation; unreachable-source behaviour.

## Acceptance Criteria

- one trait covers all four adapters with no special-casing in policy code
- artifact requests carry headers where the host requires them
- an unreachable source yields no update rather than an error path that
  could be mistaken for one
- a consumer-implemented adapter inherits channel and rollout policy with no
  additional wiring
- workspace QA passes

## Evidence Required

- adapter fixture tests including a headered request
- the documented private-GitHub limitation

## Stop Conditions

- the trait cannot express an adapter without leaking transport concerns
  into policy

## Evidence

- `UpdateSource` in `longhorn-update`: `manifest_request(channel)` and
  `artifact_request(artifact)`, both returning a `SourceRequest` of URL plus
  headers. Adapters *describe* requests; they never fetch. Card 153's
  finding makes that the right shape rather than a purity preference --
  Tauri's plugin does the fetching, verifying and installing, so a URL and
  headers is exactly what it consumes.
- `artifact_request` has a default implementation using the artifact's own
  URL unauthenticated, so public hosts and presigned URLs need no override.
- `EndpointUrl` requires HTTPS, with loopback HTTP allowed for the local
  shim. Loopback matching parses the authority rather than prefix-matching,
  so `localhost.example.com` and `127.0.0.1.example.com` are correctly
  refused.
- Adapters: `StaticJsonSource`, `GitHubReleasesSource` (covering the
  secondary-public-repository case by coordinate), `ObjectStorageSource`
  with an optional injected presigner.
- Presigning is injected, not implemented. Request signing belongs to
  whichever SDK the consumer already uses; a pure policy crate is the wrong
  place for SigV4. Without a presigner the bucket adapter behaves as a
  static host, which is correct for a public bucket.
- Private GitHub is documented as needing a proxy rather than shipped
  half-working. The acceptance test carries a `ProxiedPrivateSource` to
  prove the trait accommodates it.
- 16 source tests, including a consumer adapter inheriting rollout policy
  with no extra wiring, and an adapter that cannot downgrade transport
  security.
- fmt clean, clippy clean on both feature passes, full workspace suite green
- log: `docs/logs/2026-08/07-update-source-adapters.md`

## Next Task

Card 153 if not already underway; it does not depend on this card.
