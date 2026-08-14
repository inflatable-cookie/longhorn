# 203 Escalation Contract And Downgrade Binding

Status: complete
Completed: 2026-08-14
Owner: Tom
Roadmap: g02.021 batch 3
Governing refs: contract 018; memo 023 (M-escalation); Card 196 open decision
Depends on: Cards 200, 202 (the contract should describe the fixed extractor
and the recovery semantics)
Auto-start next card: no

## Objective

Write the `PrivilegedReplace` implementor's contract before the first host
implements the port, and close the signed-downgrade decision Card 196 left
open.

## Why this exists

`apply` extracts the verified archive into a user-writable staging dir
(`crates/longhorn-update-install/src/lib.rs:125-137`) and, on
`PermissionDenied`, hands that path to `PrivilegedReplace::replace`
(`:182-186`). The minisign signature covers the archive bytes, not the
extracted tree at move time; anything running as the user can modify staging
between unpack and the privileged move. Only `NoPrivilegedReplace` exists
today — so this is a missing contract on the port, not live exploitable code,
and now is the cheapest it will ever be to fix. The trait doc (`:54-63`) says
nothing about re-verification or protected staging.

Separately, `verify.rs:104-118` records the signed-downgrade residual (old
artifact, valid signature) and names the available mechanism — minisign's
trusted comment — but the decision was deferred to Card 196 and never taken.

## Scope

- `crates/longhorn-update-install` — trait contract, verification metadata
- contract 018 — escalation and downgrade sections
- no host implementation of `PrivilegedReplace` in this card

## Steps

1. Write the implementor's contract into the trait doc and contract 018: the
   escalated install must re-hash the staged tree against verification
   metadata, or copy from a protected location — staging mutated after
   verification must fail the move.
2. Decide what verification metadata the port receives (digest of the
   extracted root? a manifest of file hashes?) and make the type say it.
3. Close the downgrade decision: bind the version into the signed payload via
   minisign's trusted comment, or record why the residual is accepted.
   `verify.rs:104-118` already scoped the mechanism; this card takes the call.
4. Amend contract 018 in the same change; the milestone's other cards conform
   to it.

## Do Not

- Implement a host `PrivilegedReplace` here. The contract precedes the
  implementor on purpose.
- Let "no implementation exists" read as "no problem exists". The first
  implementor inherits whatever the trait doc fails to say.

## Result

The contract landed in the type, not just in prose: `PrivilegedReplace::
replace` now takes the `VerifiedArtifact` itself. The signature covers the
archive bytes, so the escalated side re-extracts — into its own protected
staging, through the now-public `extract_bundle`, which is the same bounded
extraction the unprivileged path gets. A privileged move of user-writable
content is no longer expressible through this port. The audit's sequencing
flaw came out with it: staging creation hits the unwritable parent *before*
the swap's rename does, so `apply` now escalates on `PermissionDenied` at
staging creation too — the port is reachable at all.

The downgrade decision: minisign's trusted comment is bound to the artifact
by the global signature (verified against the vendored `minisign-verify`),
so `verify_artifact` now enforces a `version:<semver>` trusted comment when
one is present — a mismatch or a malformed bound is `SignatureRejected`.
Unbound signatures (Tauri's signing emits a timestamp) verify as before;
making the comment mandatory changes what a consumer's signing step must
produce, so that mandate stays deferred and is now recorded in contract 018
with the mechanism behind it. Card 196's open decision is closed: the
mechanism is landed, enforced-when-present.

## Acceptance Criteria

- [x] the trait doc states the re-verification duty and the type carries the
  metadata it needs — the port takes the artifact, and `extract_bundle` is
  the shared bounded extraction
- [x] contract 018 describes escalation and the downgrade decision as amended
- [x] the Card 196 open decision is recorded as closed — enforced-when-present,
  mandate deferred with its reason

## Evidence Required

- the amended contract sections
- the trait doc and types
- `effigy qa` green

## Stop Conditions

Stop if trusted-comment version binding changes the artifact format consumers
already produce — that is a distribution-format decision for the operator.
