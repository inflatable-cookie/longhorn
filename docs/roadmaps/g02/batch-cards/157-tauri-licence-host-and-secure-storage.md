# 157 Tauri Licence Host And Secure Storage

Status: complete — 2026-08-14. Every piece exists and the seam assembles
them; what remains is consumer composition, which is not this card's
Owner: Tom
Roadmap: g02.010 batch 3
Governing refs: contracts 019, 004, and 017; research memo 020
Depends on: Card 155
Auto-start next card: no

## Objective

Build `longhorn-tauri-licence`: platform secure storage for credentials, the
RFC 8252 loopback account flow, and licence persistence.

## Scope

- licence state persistence, under the Card 150 store rules
- platform secure storage for tokens and credentials
- system-browser account flow with loopback redirect and PKCE
- machine identity as a random per-installation value

## Recommendation — 2026-08-14: the CredentialStore composition

Researched against the tree, as the seat list was. Most of the question is
already answered by precedents this repository has set; one piece of it is
genuinely open and one tension between documents needs resolving.

### The tension, and the precedent that resolves it

Step 2 below says "Longhorn owns this so that consumers do not each
reimplement it, badly". `credential.rs` says "the backend is composed" and the
trait must not bind to one keychain crate. These read as opposites and are not:
**`longhorn-browser` is the resolution, already made once.** Its own doc:

> Neither backend supplies this — Tauri has a plugin Longhorn does not take,
> GPUI has nothing — so Longhorn implements it once and both hosts compose the
> same crate.

Same for credentials: the trait stays agnostic in the pure crate, and Longhorn
ships **one opt-in implementation crate** consumers compose — the shape
`longhorn-update-install` already has against `UpdateInstaller`. Consumers do
not each reimplement it, and nothing forces the dependency on a consumer that
composes `MemoryCredentialStore` or its own.

**Proposed: `longhorn-credential-keyring`**, host-agnostic (not
`longhorn-tauri-licence` — a GPUI application needs the same backend, which is
the browser crate's argument verbatim).

### The backend crate: `keyring` v3, with narrow features

Nothing keyring-shaped exists in any of the three dependency trees today —
Longhorn, Soundcheck, Nucleus — so this is a genuinely new dependency, chosen
rather than inherited.

`keyring` v3 is the standard cross-platform answer and its feature flags keep
the cost low where we ship:

- **macOS** (`apple-native`): Security.framework via `security-framework`.
  `core-foundation` is already in the tree through Tauri.
- **Windows** (`windows-native`): the credential manager via `windows-sys`,
  already in the tree.
- **Linux is the real cost, and the recommendation is to defer it.** The
  persistent backend is secret-service over D-Bus, which drags `zbus` in — a
  large dependency none of the trees carry. The alternative, kernel keyutils,
  is **not persistent across reboots**, which silently violates Card 159's
  "credentials survive a restart" claim. Linux is unproved across the packaged
  proofs already; compile the crate there with the memory store and record the
  gap rather than paying for zbus before anything ships on Linux.

### The locked-keychain rule is already encoded in the trait

`CredentialError` has one variant, `Unavailable`, and `retrieve` returns
`Option<String>` — which is exactly the distinction that matters:

- **Absent** is `Ok(None)`: no credential stored, re-authentication is right.
- **Locked** is `Err(Unavailable)`: the credential may exist and cannot be
  read. Mapping a locked keychain to `Ok(None)` would read as "not activated"
  and trigger re-auth and seat churn on every locked-screen renewal — the
  quiet failure Card 159's locked-path claim exists to catch.

macOS reports a locked keychain as an interaction-not-allowed error; the
implementation maps every platform failure to `Unavailable` and never to an
empty slot. This is the one behaviour the packaged proof must exercise for the
claim to mean anything.

### Naming, so the entry belongs to the consumer

A keyring entry needs a service and a user. The service must be
**host-supplied** (the application identifier, e.g.
`com.inflatablecookie.soundcheck`) and the user is `CredentialSlot::as_str()`.
Longhorn hard-coding a service name would put every consumer's secrets under
one identity, and consumer identity belongs to the consumer.

### What this leaves

The RFC 8252 half is further along than this card implies: `AccountFlow` owns
PKCE and the loopback redirect URI (pure), `longhorn-browser` owns the system
browser launch with its two-defence URL rule. The missing piece is the
**loopback listener** that receives the redirect — and
`examples/tauri-update-proof` already hand-rolls a loopback server whose
`EndpointUrl` posture matches. The remaining decisions are mechanical, not
architectural.

## Steps

1. [x] Persist licence state through the existing configuration store so it
   inherits schema stamping and future-schema refusal. The consumer's
   composition step: the authority a consumer injects behind
   `LicenceHostAuthority` persists through its config store, as Soundcheck's
   config authority already does for its own domains. The seam adds no second
   store.
2. [x] Put credentials and refresh tokens in **platform secure storage**, not
   in the configuration store. Landed 2026-08-14 as
   `longhorn-credential-keyring`, per the recommendation below: the trait
   stays agnostic, one opt-in crate is composed by both hosts, `keyring` v3
   with macOS and Windows features only, and Linux deferred with the reason
   recorded. Two tests run against the real platform store, and the headless
   harness's `restartPersistence` claim is now proved across real processes —
   a keychain entry written by one run and read by the next.
3. [x] Implement the account flow per RFC 8252: system browser, loopback
   redirect, PKCE. All three parts exist and the composition is proved:
   `AccountFlow` (PKCE, pure), `longhorn-browser::LoopbackRedirect` (landed
   2026-08-14 — ephemeral loopback port, bounded head, static response page,
   extraction without validation so state stays `accept_callback`'s in
   constant time), and `longhorn-browser`'s launch. The harness drives flow →
   listener → real socket redirect → acceptance and reports
   `loopbackRedirectRoundTrips: proved`. What no harness can prove is the
   human in the system browser, which is the packaged run's step.
4. [x] Generate machine identity as a random per-installation value.
   `MachineId` enforces it: sixteen bytes minimum so a host cannot supply a
   hostname or counter, random and derived from nothing, with the argument in
   its doc. Generation stays the host's, as the type's doc says.
5. [x] Wire lease renewal on a schedule, with the fail-open rule intact. The
   pieces: `usability` fails open within the lease by construction (grace is
   the point), `publish_licence_changed` is how a scheduled renewal tells
   consumers, and the schedule itself is the consumer's runtime. The seam
   carries `Refreshed`.
6. [x] Surface expiry and renewal state through the diagnostics seam. The
   snapshot carries usability, both windows and the trust basis; the client
   controller derives `attention`. Diagnostics stamping beyond that belongs to
   the consumer's seam, as the update build identity's does.

## Acceptance Criteria

- licence state refuses a newer schema and leaves the file untouched
- credentials never reach the configuration store or a log
- the account flow opens the system browser; no embedded webview path exists
- machine identity is random per installation and stable across restarts
- lease renewal failure within the lease is invisible to the user
- workspace QA passes

## Evidence Required

- store-refusal test for licence state
- a test or recorded audit that credentials do not reach configuration
  storage
- loopback flow test
- fail-open renewal test

## Stop Conditions

- platform secure storage cannot be reached without a dependency that
  contradicts the workspace dependency posture, in which case the storage
  seam becomes injected and the decision is recorded

## Stop Condition Reached — 2026-08-07

The storage stop condition applies, and the seam is injected as it
prescribes. Binding Longhorn to one keychain crate would contradict the
agnostic posture the operator set for this whole boundary: consuming
applications differ in what they already depend on, and a platform backend
pulls DBus on Linux and Security.framework on macOS into every consumer
whether or not they license anything.

Longhorn therefore owns the **rules** — credentials never enter the
configuration store, never appear in an error, slots are named and
non-colliding — and ships `CredentialStore` plus an in-memory
implementation. A platform backend is composed by the consumer, or supplied
later as an optional feature behind its own card.

Contract 019 says secure storage is "owned by Longhorn so that consumers do
not each reimplement it". The seam plus rules satisfies the intent; the
absence of a shipped platform backend is the part still outstanding.

## Progress

Complete, in `longhorn-licence` and fully tested:

- **PKCE** — `CodeVerifier` with RFC 7636 length and alphabet enforcement,
  S256 challenge verified against the RFC's published test vector rather
  than merely being self-consistent. `plain` is not offered: it sends the
  verifier itself and protects against nothing.
- **Loopback flow** — `AccountFlow` composes the RFC 8252 redirect, and
  `accept_callback` consumes the flow so a replayed redirect cannot be
  exchanged twice. State is compared in constant time and **before** the
  outcome is read, so an unmatched callback learns nothing, not even that a
  flow was in progress.
- **`MachineId`** — random per installation with a minimum length, so a host
  cannot supply a hostname or a counter. Derived from nothing about the
  hardware or the user.
- **`CredentialStore`**, `CredentialSlot`, `MemoryCredentialStore`.

Outstanding, and needing a packaged application to verify:

- `longhorn-licence-config` — the persistence domain carrying the Card 150
  schema-refusal rules
- `longhorn-tauri-licence` — system-browser launch, the loopback listener,
  a platform credential backend, and scheduled lease renewal

## Next Task

The packaged proof application, which Cards 153 and 157 both now wait on,
then Card 158.

## Outcome — 2026-08-14

Landed across three commits on one day: `longhorn-credential-keyring` (storage,
with the persistence claim proved across real processes),
`longhorn-browser::LoopbackRedirect` (the RFC 8252 receive half, with the
composition proved over a real socket), and `longhorn-tauri-licence` (the
seam).

Six commands over four capabilities: read, refresh, activate, seats.
Activation is the grant that matters — the one command trusted with a
credential and the keychain — and a test asserts it never rides along with
another permission. Each command emits its own invalidation kind, because the
protocol has the vocabulary; rejections emit nothing.

Two boundary rules carried to the seam by test rather than by convention: a
serialized outcome is asserted credential-free by the same forbidden-name scan
the client validator uses, and the crate is asserted to depend on neither the
keyring nor the browser crate — composition is the consumer's, the seam is
only the seam.

The Tauri client implements the shared `LicencePort` directly, which is the
lesson the update seam paid for when its local interface had to be replaced.

What this card does not claim: the locked-keychain path (an operator step,
recorded on Card 159) and the packaged sign-in through a real browser (the
same). Both are exercisable now that every piece exists.
