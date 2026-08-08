# 162 Native Update Execution

Status: ready
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
