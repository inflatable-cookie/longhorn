# Update Source Adapters

Date: 2026-08-07
Card: 152
Roadmap: g02.009

## Result

`UpdateSource` lets a consuming application host update artifacts anywhere
and inherit channel, rollout, and floor policy unchanged. Four adapters
ship; a fifth shape is documented as needing a proxy rather than faked.

## Shape

Adapters **describe** requests rather than issuing them:

```rust
fn manifest_request(&self, channel: Channel) -> Result<SourceRequest, SourceError>;
fn artifact_request(&self, artifact: &Artifact) -> Result<SourceRequest, SourceError>;
```

A `SourceRequest` is a URL plus headers. Card 153's finding makes this the
right shape rather than a purity preference: the Tauri plugin does the
fetching, verifying, and installing, and what it consumes is exactly an
endpoint and headers. Headers are in the model because authenticated hosts
cannot express credentials in a URL — a private GitHub asset needs `Accept`
and a bearer token where a presigned S3 URL needs nothing.

`artifact_request` defaults to the artifact's own URL, unauthenticated,
which is correct for every public host and every presigned URL.

Shipped: `StaticJsonSource` (the reference shape, and what the others
degrade to), `GitHubReleasesSource`, `ObjectStorageSource`.

## Decisions

**The secondary-releases-repository case is not a separate adapter.** It
differs from plain GitHub only by coordinate — the source repository stays
private, a second public repository holds tags and binaries. Same code.

**Presigning is injected, not implemented.** `ObjectStorageSource` takes an
optional `Fn(&str) -> String`. Request signing belongs to whichever SDK the
consumer already uses, and SigV4 does not belong in a pure policy crate.
Without a presigner the adapter behaves as a static host, correct for a
public bucket.

**Private GitHub ships as documentation, not as an adapter.** Asset URLs
cannot be authenticated directly; the workable route is a proxy, which is a
consumer-implemented `UpdateSource`. Saying so beats shipping something that
half-works.

**`EndpointUrl` requires HTTPS**, with loopback HTTP allowed for the local
shim Card 153 identified. Artifacts are signature-verified regardless, but
manifests are not: a tampered manifest cannot forge an artifact, though it
can withhold one or pin an install to a stale version, so transport still
matters.

## Evidence

16 source tests. The ones that matter:

- loopback matching parses the authority instead of prefix-matching, so
  `localhost.example.com` and `127.0.0.1.example.com` are refused. Prefix
  matching here would open plain HTTP to anyone who can register the name.
- IPv6 authorities are bracketed, so the port cannot be found by splitting
  on the first colon. Caught by a test, not by review.
- a consumer adapter (`ProxiedPrivateSource`, token-authenticated) inherits
  rollout policy with no additional wiring
- an adapter cannot downgrade transport security: composing plain HTTP fails
  at the adapter boundary rather than reaching a fetch
- shipped adapters agree on channel naming, since a divergence would surface
  as a 404 on one host and not another

`cargo fmt --check` clean, clippy clean on both feature passes, full
workspace suite green. `longhorn-update` now carries 48 tests.

## Notes

Clippy's `type_complexity` caught the boxed presigner; extracted as a
`Presigner` type alias, which reads better anyway.
