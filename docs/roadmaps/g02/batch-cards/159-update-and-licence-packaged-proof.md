# 159 Update And Licence Packaged Proof

Status: ready
Owner: Tom
Roadmap: g02.009 batch 3 / g02.010 batch 3 (shared)
Governing refs: contracts 018 and 019; research memos 019 and 020
Depends on: Cards 151, 152, 155, 156
Auto-start next card: no

## Objective

Build one packaged proof application that exercises update install, restart,
and licence activation against real platform behaviour, unblocking the host
halves of Cards 153 and 157.

## Why This Exists

Cards 153 and 157 each stopped at the same boundary, for the same reason:
their remaining work cannot be verified headlessly. Two cards blocked on one
absent thing means the absent thing is a card.

The claims that need a real packaged application:

- macOS in-place bundle replacement, and the relaunch afterwards.
  tauri#11392 reports relaunch failing on macOS, with a close handler
  calling `api.prevent_close()` as a contributing factor — and Longhorn owns
  close handling, so this is ours to get right rather than an upstream
  curiosity.
- The restart interlock refusing an install while a transfer session is
  genuinely open, rather than while a test double says one is.
- A platform credential backend actually storing and retrieving through a
  real keychain, including the locked-keychain path.
- The system browser opening and the loopback listener receiving a real
  redirect.
- A non-writable installation producing the manual-download fallback rather
  than an error.

The repository already proves native-content behaviour this way; this is the
same shape applied to two new boundaries.

## Scope

- one proof application under `examples/`, following the existing
  native-content proof conventions
- update: check, download, quiesce, install, relaunch
- licence: file import, key redemption against a stub endpoint, account flow
  against a stub authorization server
- recorded evidence, not an automated CI gate — these are claims CI cannot
  make

## Steps

1. Scaffold the proof application against the existing example conventions.
2. Compose `longhorn-update` with a static-JSON source served locally, and
   drive a real install of a signed artifact.
3. Prove the restart interlock against a genuinely open transfer session,
   then against a quiescent host.
4. Record the macOS relaunch result explicitly, including whether
   `prevent_close` interferes as tauri#11392 suggests.
5. Compose `longhorn-licence` with a platform credential backend and prove
   store, retrieve, remove, and the locked-keychain path.
6. Drive the RFC 8252 flow end to end against a stub authorization server.
7. Prove the non-writable installation fallback.
8. Record every result as retained evidence; unmet claims are recorded as
   unmet rather than quietly dropped.

## Acceptance Criteria

- an update installs and the application relaunches on macOS
- an install is refused while a transfer session is open and proceeds once
  it closes
- credentials survive a restart through the platform backend
- an account sign-in completes through the system browser
- a non-writable installation falls back rather than erroring
- every unmet claim is recorded as unmet

## Evidence Required

- the proof application and its recorded run
- an explicit finding on tauri#11392 under Longhorn's close handling

## Stop Conditions

- macOS relaunch cannot be made reliable under Longhorn's close handling, in
  which case the finding is the deliverable and the interlock gains a
  documented manual-relaunch path

## Next Task

Finish Cards 153 and 157 against the proof, then Cards 154 and 158.
