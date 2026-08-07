# Licence Model And Entitlement Evaluation

Date: 2026-08-07
Card: 155
Roadmap: g02.010

## Result

`longhorn-licence` carries the licence shape and its evaluation. It never
fetches, never persists, and never enforces. 28 tests.

## Shape

- `LicencePayload` — product, entitlements, and two independently optional
  windows: `use_until` and `update_until`, plus `lease_until`.
- `TrustBasis` — `OfflineSignature { key_id }` or
  `RemoteAssertion { checked }`.
- `VerifiedLicence` — constructible only through `verify` or an adapter
  declaring a remote assertion. No struct literal path.
- `Entitlements` — a map of opaque `EntitlementId` to `Limit`.
- `usability(licence, now, guard, grace) -> Usability`.
- `SignedLicence` and `verify`, Ed25519 over the received bytes.
- `Timestamp` and `Span` as plain integers, so the crate needs no date
  dependency and every expiry is testable without waiting.

## Decisions

**The payload travels as bytes, and the signature covers those bytes.**
Verification checks the signature *then* parses. This closes the classic
hole in these schemes: verifying a re-serialisation rather than what was
received turns any canonicalisation difference — field order, number
formatting, whitespace — into a forgery. A test signs a payload with
trailing whitespace and asserts it verifies and parses, which a
re-serialising implementation would fail.

**Grace differs by trust basis, and the difference is an invariant.**
`GracePolicy::new` clamps remote-assertion grace to signature grace, so a
consumer cannot configure their way around it. From identical payloads at
day 40, a signature licence is in grace and a remote one has lapsed.

**Absent and unlimited are distinct entitlement answers.** `limit()` returns
`Option<Limit>`, so a caller cannot mistake "not sold this" for "sold this
without a cap".

**Ordering in `usability` is deliberate.** The clock guard runs first,
because every window comparison below it is meaningless if the clock is not
trusted. The use window runs before the lease, because "your subscription
ended" is truer and more actionable than "revalidation failed" on an expired
subscription.

**Fail-open is structural, not a branch.** Nothing in the crate consults
reachability. A backend outage is simply a lease not yet renewed, and inside
the lease that is indistinguishable from a healthy licence — so there is no
code path in which an outage disables a paying customer.

**Grace does not warrant attention.** `Usability::warrants_attention`
returns false for `InGrace`, because a renewal inside its tolerance is not
something the user can act on. Surfacing it would train people to ignore
licence messages.

**Clock tolerance defaults to one day.** An NTP correction, a timezone
mistake, or a dead CMOS battery is not abuse. The guard is deliberately
partial: it stops casual regression and does not pretend to stop determined
abuse, because licensing is not a security boundary.

## Evidence

`tests/models.rs` expresses subscription, perpetual-with-maintenance, trial,
freemium, and a site licence Longhorn never anticipated — with no branch on
a product type and no enum naming a business model anywhere in the crate.
That file is the acceptance criterion made executable.

`tests/policy.rs` covers the trust-basis grace distinction, the clamp,
fail-open, use-window precedence, clock regression at three tolerances,
tampering, wrong key, malformed signature, the whitespace round trip, and
signed-but-not-a-licence bytes.

`cargo fmt --check` clean, clippy clean on both feature passes, full
workspace suite green.

## Notes

No public function enforces, disables, or degrades anything; the surface is
`verify`, `usability`, and entitlement queries. That was an acceptance
criterion and it held without pressure.

`ed25519-dalek` joins the workspace dependencies. Crate count 39 → 40.
`docs/reference/api-surface.md` regenerated,
`docs/architecture/package-topology.md` updated.
