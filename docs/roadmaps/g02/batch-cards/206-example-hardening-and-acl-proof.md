# 206 Example Hardening And ACL Proof

Status: complete
Completed: 2026-08-14
Owner: Tom
Roadmap: g02.022 batch 3
Governing refs: contract 010; contract 012; memo 023 (L-examples, ACL gap)
Depends on: none (adopts Cards 204/205 outcomes if they have landed)
Auto-start next card: no

## Objective

The example applications a consumer copies start from the strong posture, and
the capability templates are behavior-tested, not just drift-tested.

## Why this exists

`examples/tauri-update-proof` and `examples/tauri-windowing-proof` ship
`"withGlobalTauri": true`, CSP `script-src 'self' 'unsafe-inline'`
(`tauri.conf.json:23` both), and capabilities listing only `core:default` with
no Longhorn permission scoping. The prototypes use strict CSP
(`script-src 'none'`). Examples are what consumers copy; these normalize the
weakest posture in the repo. Separately, `tests/capabilities.rs` is a
file-content drift test and `host_assembly.rs` uses a mock runtime — whether
Tauri's ACL actually refuses a denied window is delegated, untested
end-to-end.

## Scope

- the two packaged proofs' `tauri.conf.json` and capabilities
- one packaged ACL assertion (which proof hosts it is a step-1 choice)
- no changes to the capability templates themselves unless the proof shows
  them wrong

## Steps

1. Harden both proofs: drop `withGlobalTauri`, strict CSP matching the
   prototypes, capabilities declaring the Longhorn permission sets the proof
   actually needs. The windowing proof's `core:window:allow-start-dragging`
   grant gets a comment saying why.
2. Add the ACL enforcement proof: a window denied `allow-longhorn-bridge-
   mutate` issues a mutate command and is refused — asserted in the packaged
   app, not the mock runtime.
3. If any example *needs* the weak posture to run, that need is a finding —
   record it; do not silently keep the hole.
4. Note the pattern in the getting-started or composition guide if the
   capability setup is more than mechanical.

## Do Not

- Touch `.github/workflows/` without explicit approval.
- Treat the proofs as disposable. They are release evidence; their posture is
  part of what they prove.

## Acceptance Criteria

- [x] both proofs run strict CSP — `default-src 'self'`
- [ ] both proofs drop `withGlobalTauri` — **not done; named residual in
  Result**
- [x] both proofs declare scoped permissions over their own commands
- [x] a denied window is proved refused — via Tauri's real IPC router against
  the proofs' own capability files (`src-tauri/tests/acl.rs`), one layer
  below a physical packaged run
- [x] any residual weak posture is named with its reason

## Evidence Required

- the updated configs and capabilities files
- the ACL proof output
- `effigy qa` and the affected proof selectors green

## Stop Conditions

Stop if strict CSP breaks a proof in a way that needs Tauri-level changes —
that is upstream of this card.
