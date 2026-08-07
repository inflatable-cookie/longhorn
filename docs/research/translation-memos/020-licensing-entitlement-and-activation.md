# 020 Licensing, Entitlement, And Activation

Status: complete and promoted
Owner: Tom
Updated: 2026-08-07
Promotes: contract 019; the g02.010 milestone. Touches contracts 004
(licence state is persisted state) and 018 (adapter posture, install
identity).

## Prompt

Consuming applications need to sell licences. At minimum serial keys and
remote account activation, but the purchase model differs per application
and the backend is the application's to choose. Decide what an agnostic
framework can own without deciding the product for its consumers.

## Sources

Prior art in the desktop licensing field: signed offline licence files,
partial-key-verification serial schemes and their failure mode, activation
slots and seat management, RFC 8252 for native-app OAuth. Vendor models
surveyed for interface shape only, not for adoption: Keygen (self-hostable,
signed Ed25519 licences), Lemon Squeezy and Paddle (merchant of record with
licensing attached), Cryptolens. Workspace read at `b83ff75b`: no licensing
surface exists in any of the 39 crates or 18 packages.

## Findings

### Licensing is not a security boundary, and designing as though it is costs more than it saves

The check runs on hardware the user controls. Sufficient motivation defeats
any client-side scheme, so the achievable goals are: frictionless for honest
customers, mildly inconvenient to share casually, and flexible enough to
survive a pricing change. A design that optimises for unbreakability
produces DRM that punishes paying customers and is broken anyway.

This is the finding that orders every decision below. Where a choice trades
customer friction against piracy resistance, it resolves toward the
customer.

### A short typeable serial key cannot be self-verifying

An Ed25519 signature is 64 bytes — 103 base32 characters before any payload.
No one types that. The historical alternative, partial key verification, is
keygen-able permanently for every key ever issued the moment one person
reverses it; that is a delayed total failure, not a tradeoff.

So a serial key must be a **redemption token**: short, random, revocable,
exchanged for a licence. The key identifies an order; it does not carry
authority. That makes serial keys a *delivery mechanism* rather than a
licensing system, which in turn means they sit behind the same adapter
boundary as everything else.

### Trust basis genuinely differs by backend, and flattening it is a real bug

A signed licence file is verifiable offline, indefinitely, with no network.
A hosted-service API response is a TLS assertion that can only be **cached**;
it cannot be re-verified later without the network that granted it.

A framework that models both as one `Licence` will grant offline grace on a
basis incapable of surviving being offline. So a verified licence records
how trust was established, and grace policy can require an offline-verifiable
basis. This is what keeps the adapter boundary honest rather than reducing
every backend to its weakest claim or overstating its strongest.

### One window split covers every purchase model

Rather than enumerating subscription, perpetual, trial, and freemium, a
licence carries two independently optional windows: **until when may this be
used**, and **until when may updates be taken**.

| Product | use window | update window |
| --- | --- | --- |
| subscription | lease date | same |
| perpetual with maintenance | none | purchase + term |
| trial | date | none |
| freemium | none | none |

Four products, no product-specific code, and a consumer can express a fifth
without a Longhorn change. The split also connects licensing to contract 018
directly: the update window is exactly the question the updater must ask
before offering a release.

### Entitlements must be opaque, and Longhorn must never enumerate features

Modelling `licensed: bool`, or even an edition enum, makes every pricing
change a code change and a release. Entitlements are consumer-defined
strings with consumer-defined limits; Longhorn evaluates presence and bounds
and knows nothing about meaning.

Corollary: **Longhorn answers "entitled?" and never enforces.** A framework
that unilaterally disabled windows or refused to save would be intolerable
to build on. What absence of an entitlement *means* is the application's
decision.

### The pain is operational, not cryptographic

Nothing below concerns the signature scheme, and all of it drives support
cost:

- **"I got a new laptop"** is the dominant licensing support ticket
  everywhere. Self-service deactivation of an activation slot is not a
  refinement; it is the feature.
- **Machine identity** must be a random per-installation value, not a MAC
  address or hardware serial — those are privacy-hostile and unstable under
  VMs and adapter churn. The update work's install identity is the same
  shape and the same reasoning.
- **Offline operation** requires a lease with grace. An application that
  stops working on a plane is a bug whatever the agreement says.
- **Lease length is the revocation window.** A signed offline licence cannot
  be recalled; renewal is the only revocation. Short lease, tight
  revocation, worse offline story — that is the whole dial.
- **Clock regression** defeats offline expiry. Persisting the highest
  timestamp seen and refusing a large backwards jump costs almost nothing
  and stops casual abuse.
- **Server unreachable must fail open** within the lease. A licensing
  outage that bricks paying customers is a self-inflicted incident costing
  more than the piracy it prevents.

### Backends stay consumer-owned

Operator decision. Longhorn ships no licence server and takes no position on
merchant of record, tax jurisdiction, or payment provider. Adapters are the
whole integration surface, mirroring contract 018's `UpdateSource`, so a
consumer can start on a hosted service and move later without changing
anything but composition.

## Decision

Compile contract 019 and one milestone, g02.010, in four cards:

1. `longhorn-licence` — the pure core: verified licence model, trust basis,
   entitlement evaluation, the two windows, lease and grace, clock-regression
   detection;
2. the `ActivationSource` adapter interface plus reference adapters for
   signed-file import and generic token redemption, with hosted backends
   documented as consumer-implemented;
3. `longhorn-tauri-licence` — secure credential storage, the RFC 8252
   loopback flow, and persistence;
4. `packages/licence` — activation, seat management, and expiry surfacing.

## Open Questions And Planning Gaps

- **Signing key custody** for consumers who choose offline signed licences,
  with the same rotation problem as the updater's minisign key: one embedded
  public key, so rotation is ship-accept-wait-switch. Consumer-owned; noted
  because the two key-custody problems should be solved once, together.
- **Reinstall-farm detection** via coarse hardware fingerprinting is
  deliberately not in the first pass. It trades privacy for a class of abuse
  not yet observed, and adding it later is additive.
- **EULA acceptance and seat terms** are legal artefacts beside this work,
  not licence-key mechanics.

## Consumer Exposure

Additive capability: two crates and one package, inert until composed. No
existing surface changes. Licence state is persisted state, so it inherits
the cross-channel store rules proved in Card 150 — a nightly build must not
write a licence store production cannot read.
