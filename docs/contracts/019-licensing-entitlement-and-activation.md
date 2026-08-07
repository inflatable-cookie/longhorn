# 019 Licensing, Entitlement, And Activation

Status: active compiled boundary
Owner: Tom
Updated: 2026-08-07
Architecture: `../architecture/system-architecture.md`
Research: `../research/translation-memos/020-licensing-entitlement-and-activation.md`

## Boundary

Longhorn owns the *shape* of a licence and the *evaluation* of it.
Applications own where a licence comes from, what its entitlements mean, and
what happens when one is absent.

Longhorn ships no licence server, and takes no position on payment provider,
merchant of record, or tax jurisdiction.

## Licence Model

- A licence carries entitlements, limits, an optional **use window**, an
  optional **update window**, and a lease.
- The two windows are independent. Subscription, perpetual-with-maintenance,
  trial, and freemium are all expressible without Longhorn naming any of
  them, and a consumer may express a model Longhorn has not anticipated.
- The update window is the question the updater asks before offering a
  release, binding this contract to contract 018.
- Entitlements are opaque consumer-defined identifiers. Longhorn evaluates
  presence and bounds and never enumerates, interprets, or reserves them.

## Trust Basis

- A verified licence records **how** trust was established, because backends
  differ in kind and not merely in transport.
- An offline signature is re-verifiable at any later moment without a
  network. A remote assertion is only as good as its cache and cannot be
  re-established offline.
- Policy may require an offline-verifiable basis. Offline grace must never
  be granted on a basis incapable of surviving being offline.
- Longhorn ships signature verification. An adapter that establishes trust
  another way declares that basis honestly; it may not present a remote
  assertion as a signature.

## Acquisition

- Acquisition is entirely adapter-shaped, mirroring contract 018's
  `UpdateSource`. Serial redemption, account activation, and licence-file
  import are three delivery mechanisms producing one verified licence.
- A serial key is a redemption token, never a self-verifying artifact. Short
  typeable keys cannot carry a signature, and schemes that pretend otherwise
  fail permanently on first reverse.
- Native-application account flows use the system browser with a loopback
  redirect and PKCE. Embedded webviews are not an accepted flow.
- Longhorn ships reference adapters for signed-file import and generic token
  redemption. Hosted backends are consumer-implemented.

## Enforcement

- **Longhorn answers "entitled?" and never enforces.** It does not disable
  windows, refuse saves, or degrade behaviour on its own authority.
- What the absence of an entitlement means is the application's decision,
  including whether it means anything at all.

## Activation And Identity

- Machine identity is a random per-installation value, never derived from
  hardware serials, MAC addresses, or user identity.
- Self-service deactivation of an activation slot is required, not optional.
- Seat accounting is the backend's; Longhorn carries the slot identity and
  surfaces the state.

## Offline, Lease, And Failure

- A licence remains usable offline until its lease lapses, with grace beyond
  it.
- Lease length is the revocation window. A signed offline licence cannot be
  recalled; renewal is the only revocation, and the tradeoff is explicit.
- An unreachable backend **fails open** within the lease. A licensing outage
  must never disable a paying customer.
- A large backwards clock movement is refused rather than trusted.

## Persistence

- Licence state is persisted state and inherits the cross-channel store
  rules: it records the schema that wrote it and refuses a newer one.
- Credentials and tokens use platform secure storage, owned by Longhorn so
  that consumers do not each reimplement it.

## Non-goals

- A Longhorn licence server, payment handling, or tax handling.
- Signing key custody, which is consumer-owned.
- Hardware fingerprinting and reinstall-farm detection.
- EULA presentation and acceptance recording.
- Obfuscation, anti-tamper, or anti-debugging. Licensing is not a security
  boundary and will not be presented as one.
