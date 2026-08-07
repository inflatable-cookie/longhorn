# 152 Update Source Adapters

Status: ready
Owner: Tom
Roadmap: g02.009 batch 2
Governing refs: contract 018; research memo 019
Depends on: Card 151
Auto-start next card: no

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

## Next Task

Card 153 if not already underway; it does not depend on this card.
