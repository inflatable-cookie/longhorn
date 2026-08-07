# 019 Application Update And Release Channels

Status: complete and promoted
Owner: Tom
Updated: 2026-08-07
Promotes: contract 018; the g02.009 milestone. Touches contracts 004
(store compatibility), 012 (distribution), and 017 (restart interlock).

## Prompt

Longhorn consumers need in-app update: publish a release, have the running
app notice it, offer it in the UI, download it, and replace the running
instance. Decide what Longhorn owns, how release artifacts are hosted when
the source repository is private, and how production, beta, and nightly
lines coexist.

## Sources

Tauri v2 updater plugin documentation and changelog, the `tauri-apps`
discussion on private-repository releases, and prior art in Sparkle
(macOS appcast, EdDSA, channels from v2), `electron-updater`
(`latest.yml` per channel, provider abstraction), and the staged-rollout
models used by Chrome and Firefox. Workspace state read at `0cc0a333`:
no updater surface exists in any of the 38 crates or 18 packages.

## Findings

### The mechanism is already solved; the policy is not

`tauri-plugin-updater` performs download, signature verification, and
in-place replacement on all three platforms. Artifacts are signed with a
minisign keypair whose public half is compiled into the application. The
updater rejects anything that fails verification, so **the artifact host
does not have to be trusted**. That single property is what makes host
choice a logistics question rather than a security one, and it is the
reason Longhorn must never reimplement verification.

What has no answer today is channel policy, rollout policy, host
abstraction, and — the only part that is Longhorn-shaped — whether a
restart is safe at the moment the user accepts one.

### GitHub cannot serve public releases from a private source

Release assets inherit repository visibility; there is no per-release
override. Authenticated asset download needs an asset-ID API call plus an
`Accept: application/octet-stream` header, so the plain browser URL cannot
be handed to a downloader. The established workarounds are a separate
public releases repository, object storage, or an authenticating proxy.
This constrains the adapter interface (below) but does not decide it.

### A single bundle identity makes application data a shared surface

Operator decision: nightly ships under the production bundle identifier
rather than as a side-by-side install, because separate identifiers
fragment storage, caches, and window state and force migration tooling
between an operator's own applications.

The consequence lands squarely on Longhorn, which owns all four persistent
stores — configuration, settings, history, and history-tree. A nightly
build and a production build now read and write the same files. The
failure sequence is:

1. a nightly build writes a store under a newer schema;
2. the install returns to the production channel, by user choice or by a
   nightly rollback;
3. the production build opens a store written by a schema it has never
   seen, and best-effort parsing silently drops the fields it does not
   recognize before writing them back.

The rejoin path is not an edge case: every nightly user eventually takes
it, because a nightly install rolls onto production automatically once
production reaches the same version. No store currently records the schema
that wrote it, so the failure is undetectable rather than merely possible.

### Client-side rollout is what keeps hosting dumb

Staged rollout is wanted from the first release. Most candidate hosts are
static object storage with no request-time logic, so eligibility has to be
decided on the client: the manifest carries a rollout fraction and seed,
and the client hashes a persisted random install identifier against the
seed. Deterministic per install, so widening a rollout never revokes an
offer already made, and the operator widens it by editing one field.

Two overrides matter more than the mechanism. A `minimum_version` floor
must update every install below it regardless of rollout — this is the
security-fix lever, and it is the reason to build rollout properly rather
than approximately. And an explicit user-initiated check must bypass
rollout, because the user asking is the user opting in.

### Semver ordering gives channel rejoin for free

Under one bundle identity, production is `1.2.3`, beta is `1.3.0-beta.4`,
and nightly is `1.3.0-nightly.20260807`. Prerelease ordering makes a
nightly install strictly older than the production release it anticipates,
so nightly users rejoin production at `1.3.0` with no special handling.

The same ordering means a channel switch is frequently a downgrade the
updater will refuse: an install on `1.3.0-nightly.x` that selects
production sits ahead of production `1.2.9` and receives nothing until
`1.3.0` ships. That is correct, and it is indistinguishable from a broken
updater unless the client surface says so explicitly.

### Restart safety is the part only Longhorn can provide

Longhorn owns window lifecycle, surface transfer sessions, history, and
async operations. An update that relaunches during a transfer commit or a
pending flush is data loss. The machinery to prevent it already exists —
`shutdown_flush`, the teardown phases in contract 017, and the poison and
restore consistency work from Card 144 — but nothing connects it to an
installer. A consuming application cannot write this interlock, because it
does not know what is in flight.

## Decision

Compile contract 018 and one milestone, g02.009, in five cards:

1. store schema stamping with forward-refusal, which blocks any nightly
   build from shipping at all and therefore lands first;
2. `longhorn-update` — channel policy, manifest model, rollout gate,
   version comparison, deferral state; no Tauri dependency;
3. an `UpdateSource` adapter trait with default implementations for static
   JSON, public GitHub releases, a secondary public releases repository,
   and S3/R2 with optional presigning;
4. `longhorn-tauri-update` — the restart-readiness interlock against the
   lifecycle coordinator, then plugin wiring and install;
5. `packages/update` — the client surface, including channel selection and
   the ahead-of-production state.

Adapters supply the manifest and an artifact request; Tauri performs
download, verification, and installation throughout.

## Open Questions And Validation Needs

- **Whether Tauri will install a specifically chosen artifact**, or only
  what its configured endpoint returns. If endpoint-only, the resolved
  manifest is served to it over a loopback endpoint bound to `127.0.0.1`
  with a one-shot nonce. Signature verification still gates the install
  either way, so this is a crate-shape question and not a security one.
  Card 153 settles it before building.
- **How `installMode` and the macOS in-place bundle replacement interact
  with Longhorn's teardown ordering.** Card 153.
- **Minisign key custody and rotation.** Losing the private key strands
  every installed application permanently — each user reinstalls by hand.
  Only one public key is embedded per build, so rotation requires shipping
  a version that accepts the successor, waiting for adoption, then
  switching. Operator-owned; recorded in the planning-gaps register, not
  resolved here.
- **Update failure on non-writable installations** (Homebrew casks,
  administrator-installed copies) needs a manual-download fallback rather
  than an error path.
- **Rollback has no Tauri mechanism.** Staged rollout limits blast radius;
  it does not undo. A recovery story stays an open gap.

## Consumer Exposure

This milestone adds two crates and one package, which the nucleus boundary
verifier will reject until nucleus updates. Unlike the g02 remediation
runway, this work **cannot** stay internal to Longhorn. Sequencing with
nucleus is a precondition of Card 151, not a closeout detail.
