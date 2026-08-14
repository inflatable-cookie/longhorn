# g02.021 Update Supply-chain Hardening

Status: ready
Owner: Tom
Updated: 2026-08-14
Governing refs: contract 018; memo 023; memo 019
Depends on: g02.009 (complete)

## Outcome

The update install path's stated properties are true. Extraction is bounded
against link entries, not just names; the loopback exception cannot be spoofed
with userinfo; a crash mid-swap recovers instead of bricking the install; the
escalated path has a written contract before its first implementor; and the
signed-downgrade decision deferred from Card 196 is closed.

## Generation Runway

Memo 023's highest-severity code findings all sit on the update path g02.009
built. Contract 018:51 claims bounded extraction the code does not deliver —
the milestone either makes the claim true or amends the claim, and the honest
answer is the former: the verifier is already type-forced, and leaving the
extractor weaker than the verifier wastes the design.

## Planning Gaps

None. All four cards execute inside contract 018 as amended. The
signed-downgrade mechanism (minisign trusted comment) is already identified in
`verify.rs:104-118`; Card 203 takes the decision, it does not research it.

## Execution Plan

### Batch 1. Input boundaries

- [x] [Card 200](batch-cards/200-extraction-that-is-bounded.md): link-target
  validation or `unpack_in` semantics; hostile symlink/hardlink fixtures join
  the shared conformance suite; random staging suffix; orphan sweep.
  **Landed 2026-08-14** — all three layers: `bounded` on names *and* link
  targets, `assert_inside` canonical ancestor checks, tar's `unpack_in` as
  backstop; four hostile fixtures in the shared suite.
- [x] [Card 201](batch-cards/201-endpoint-authority-parsing.md): userinfo
  stripped before the loopback match; `manifest.channel == build.channel`
  checked at evaluate; the licence `ActivationUrl` parsing reviewed against
  the same fix. **Landed 2026-08-14** — last-`@` userinfo strip, new
  `ChannelMismatch` rejection code, licence side confirmed unreachable by the
  class.

### Batch 2. Crash honesty

- [ ] [Card 202](batch-cards/202-install-atomicity-and-recovery.md): startup
  recovery sweep for `*.longhorn-previous`; a mid-swap kill test; resource
  bounds on fetch and extraction; `authority_epoch` made real or removed.

### Batch 3. Escalation and downgrade

- [ ] [Card 203](batch-cards/203-escalation-contract-and-downgrade-binding.md):
  the `PrivilegedReplace` implementor's contract (re-verify or protected
  staging) written before any host implements the port; version bound into the
  signed payload via minisign trusted comment.

## Dependency Shape

```text
memo 023 (H1, H2, M-swap, M-size, M-escalation, L-epoch, L-channel)
 └─ 021 update supply-chain hardening
     ├─ 200 bounded extraction   (independent)
     ├─ 201 endpoint authority   (independent)
     ├─ 202 atomicity + bounds   (independent)
     └─ 203 escalation contract  (writes the contract 018 amendment last)
```

All four cards are independent at the code level; 203 lands last because it
amends the contract the others conform to.

## Goals

- [ ] contract 018:51's bounded-extraction claim is true for link entries
- [ ] no authority string classifies a remote host as loopback
- [ ] a kill between the two renames is recoverable on next launch
- [ ] `PrivilegedReplace`'s first implementor inherits a re-verification duty,
  not a gap

## Acceptance Criteria

- [ ] the shared install conformance suite includes hostile symlink, hardlink,
  and absolute-link-name fixtures and any second installer inherits them
- [ ] `http://127.0.0.1:80@evil.example/` and `http://[::1]@evil.example/` are
  rejected, with tests
- [ ] fetch and extraction carry byte bounds; a hostile host cannot exhaust
  memory or disk pre-verification
- [ ] the Card 196 open decision is closed and recorded in contract 018

## Explicit Non-goals

- A rollback *feature*. Contract 018:180 keeps user-facing rollback a non-goal;
  Card 202 restores crash atomicity, not revert.
- A production `ArtifactFetch`. The host performs downloads; Longhorn bounds
  them.

## Next Task

Card 202. Crash honesty: recovery for a kill mid-swap, byte bounds on fetch
and extraction, and the `authority_epoch` decision.

## Planning Checkpoint

After Batch 1. If link-target validation turns out to require reimplementing
extraction rather than wrapping it, that is a shape decision worth one pause —
the conformance fixtures land either way.
