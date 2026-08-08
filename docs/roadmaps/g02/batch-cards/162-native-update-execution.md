# 162 Native Update Execution

Status: in progress — contract and suite landed; implementations outstanding
Owner: Tom
Roadmap: g02.012 batch 2
Governing refs: contracts 018 and 020; research memo 021
Depends on: none
Auto-start next card: no

## Objective

Implement update execution for hosts with no updater plugin — download,
signature verification, unpack, install, relaunch — under the same
behavioural contract the Tauri plugin path satisfies.

## Rationale

Contract 018 was amended on 2026-08-08. Authorization-only was correct while
Tauri was the only host; it became wrong when GPUI became first-class,
because no plugin exists there. The alternative is every product
implementing minisign verification and macOS bundle replacement itself:
security-sensitive code, duplicated per application, which is what a
framework exists to prevent.

## Scope

- native installer for hosts with no plugin
- one shared conformance suite both paths satisfy
- the system browser opener contract 019's RFC 8252 flow needs

## Steps

1. Write the behavioural contract first, as a conformance suite, before
   either implementation is touched. Two implementations without a shared
   suite is a fork, not an adapter.
2. Run the suite against the Tauri plugin path to establish the baseline.
   Anything the plugin does that the suite does not capture is either
   contract or accident, and this is where that gets decided.
3. Implement the native path: download, verify, unpack, replace, relaunch.
   Verification is not optional and has no configuration — an installer that
   does not verify is not an installer.
4. Reuse `longhorn-update`'s policy unchanged. This card adds execution, not
   decisions.
5. Keep the restart interlock host-agnostic. `UpdateGate::authorize` already
   answers for both paths.
6. Add the system browser opener as a host capability.

## Steps 1 Landed — 2026-08-08

The contract and its conformance suite exist before either implementation,
as the card required.

`UpdateInstaller::apply(version, artifact, signature) -> Result<Applied,
InstallFailure>` in `longhorn-update`, with `run_conformance` returning one
outcome per claim.

**The contract is deliberately coarse.** Tauri's plugin performs download,
verification, and replacement as one opaque call, so a contract demanding
separable steps could not be satisfied by it. What both implementations can
promise is observable behaviour: what reaches disk, and what is reported.

**The suite reports every failure rather than the first**, so a report names
everything an implementation gets wrong instead of inviting fix-and-rerun-blind.

**Seven meta-tests prove the suite catches what it claims.** A suite nobody
has seen fail is a suite nobody should trust, so each feeds it a
deliberately wrong implementation and asserts the specific claim goes red:
an installer that skips verification, one that rejects for the wrong reason,
one that conflates an unusable artifact with a signature failure, and one
that applies the wrong version.

Two distinctions the suite enforces that a naive implementation loses:

- **Rejecting is not enough.** A signature failure reported as a generic
  fault invites a retry loop against an attacker-supplied artifact, so
  `SignatureRejected` is not retryable and the suite checks the reason, not
  just the refusal.
- **"We do not trust this" and "we trust it and cannot use it" are different
  messages**, and only one is a security event. `MalformedArtifact` is
  separate from `SignatureRejected`.

### Format decision — minisign, from the Tauri side

Tauri's plugin verifies with **minisign**, via `minisign-verify ^0.2`.

The native implementation must use the same format and the same key.
Anything else means two signatures per artifact, two keys, and two signing
steps per release — for one product shipping to two hosts. One release, one
signature, two verifiers that agree by construction.

This is a constraint on the native implementation, not a choice it gets to
make.

## Outstanding

- the native implementation: minisign verification, unpack, atomic
  replacement, relaunch
- the baseline conformance run against the Tauri plugin path, which needs a
  packaged application and therefore shares Card 159's blocker
- the system browser opener

## Acceptance Criteria

- both paths satisfy one conformance suite
- an unverified artifact is refused on every host, under every configuration
- policy is untouched: no channel, rollout, or floor logic is duplicated
- the non-writable-installation case is classified, not surfaced as failure
- macOS bundle replacement and relaunch are proved or recorded as unproven

## Evidence Required

- the conformance suite, and both paths passing it
- a tamper test refused by the native path
- the recorded baseline of plugin behaviour the suite does not capture

## Stop Conditions

- the native path cannot satisfy a claim the plugin satisfies, in which case
  the divergence is stated in contract 020 rather than hidden

## Next Task

Card 163, if not already underway.
