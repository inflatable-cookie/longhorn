# 200 Extraction That Is Bounded

Status: ready
Owner: Tom
Roadmap: g02.021 batch 1
Governing refs: contract 018; memo 023 (H1)
Depends on: none
Auto-start next card: no

## Objective

Make `crates/longhorn-update-install/src/lib.rs:34-36`'s bounded-extraction
claim true for link entries, and prove it with hostile fixtures any second
installer inherits.

## Why this exists

`unpack` (`lib.rs:194-233`) validates only entry *names* via `bounded()`
(`:240-259`) and then calls `entry.unpack(&destination)`. The vendored tar
crate's `unpack` skips `validate_inside_dst` — that guard lives only in
`unpack_in`, which this code does not use. Hard links with absolute or `../`
targets are used verbatim; a two-entry archive (`link -> /target/dir`, then
`link/payload`) passes `bounded()` and writes outside staging; a top-level
symlink is returned as `root` and renamed onto the install target. The
existing escape test (`tests/install.rs:192-211`) covers only textual `../`.

## Scope

- `crates/longhorn-update-install`: extraction path, staging naming, cleanup
- the shared install conformance suite (`tests/install.rs`)
- contract 018:51 stays as written — the code moves to the claim

## Steps

1. Choose the mechanism: reimplement entry dispatch over `unpack_in`
   semantics, or validate link targets before `entry.unpack` (reject absolute
   and `..` link names; canonicalize destination parents before writing).
   Record the choice and why in the crate header.
2. Implement. Every entry type — file, directory, symlink, hardlink — gets a
   stated rule.
3. Add hostile fixtures to the shared conformance suite: symlink escape,
   hardlink escape, absolute link name, link-then-payload-through-link,
   top-level symlink root. They must run per installer implementation, the
   way verification was made structural.
4. Random suffix on `.longhorn-update-{version}` staging (`lib.rs:125`); a
   pre-planted symlink at the staging path must fail, not redirect writes.
5. Startup sweep: failed installs of versions other than the in-flight one
   leave staging dirs forever today (`lib.rs:127,131,137,152,167`); sweep
   stale staging and leftover backups on apply.

## Do Not

- Weaken contract 018:51 to match the code. The claim is right; the code is
  wrong.
- Blame the precondition. "Minisign-verified first" bounds the attacker to the
  signing key, but the design's own premise is that a signature proves origin,
  not good intent (`lib.rs:238-239`).

## Acceptance Criteria

- [ ] no archive entry of any type can cause a write outside staging
- [ ] the hostile fixtures live in the shared conformance suite, not one
  installer's private tests
- [ ] a pre-created staging path cannot redirect extraction
- [ ] a failed install leaves nothing behind after the next apply

## Evidence Required

- the mechanism choice and its reason, in the crate header
- the new conformance cases, failing before and passing after
- `effigy qa` green

## Stop Conditions

Stop if correct link handling requires forking the tar crate's entry logic
rather than composing it — that is a dependency-shape decision, not a card.
