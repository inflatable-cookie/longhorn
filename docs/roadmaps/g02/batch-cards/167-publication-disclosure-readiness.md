# 167 Publication Disclosure Readiness

Status: ready
Owner: Tom
Roadmap: g02.014 batch 2
Governing refs: contract 012; Card 166
Depends on: none
Blocks: making either repository public

## Objective

Make Longhorn and Poodle safe to publish as public repositories. Third-party
identity leaves the repository; the operator's own product names stay.

## The Decision, 2026-08-09

Nucleus, Loophole, Soundcheck, Jetstream and Figmatic are the operator's own
products. They stay named — a framework with visible adopters is better
evidence than one with anonymised ones.

**The acowtancy family is client work and must not appear.** That is
`acowtancy`, `bovine`, `bovine-accelerator`, and the reverse-DNS
`com.acowtancy.bovine-accelerator`. `dairy`, `froyo` and `cream` were checked
and appear nowhere in Longhorn or Poodle; they exist only in their own private
repositories.

## Measured 2026-08-09

Longhorn, 146 files:

| Area | Files |
| --- | --- |
| docs | 89 (32 logs, 29 roadmaps, 11 research, 8 architecture, 5 contracts, 2 guides, 1 spec, 1 reference) |
| scripts | 20 |
| examples | 20 |
| crates | 9 |
| fixtures | 8 |

Poodle: 7 files.

Two other disclosure classes, unrelated to clients:

- `/Users/tom` in 18 files, all under `examples/` and `prototypes/`
- `audio.infiniteloop.soundcheck` style identifiers in crate `#[cfg(test)]`
  modules, which disclose an org identifier that is not otherwise present

No credentials, keys, emails or customer data were found in the tree or in the
history's added-file names.

## No Live Gate Depends On Any Of It

This is what makes the card small. `effigy.toml` references none of the
acowtancy material. `verify-bovine-card120.ts` and `verify-bovine-card121.ts`
are dormant historical scripts belonging to completed cards; they are in no
aggregate, and `proof:artifacts` does not run them.

So nothing that currently gates a release reads a client repository, and
removing that material cannot break a gate.

## What Moves Where

**Third-party evidence leaves the repository.** The two dormant verifiers and
their frozen fixtures (`fixtures/migration/bovine-card120`,
`bovine-card121`) assert a client's application id, storage leaf and
dependency graph. Move them to a gitignored `private/` overlay with a
committed `private/README.md` stating that consumer-specific migration
evidence for third-party products is held outside the public repository.

Deleting them would be simpler and loses the evidence for two completed cards;
moving keeps it recoverable and keeps the public tree honest about the fact
that it is not the whole record.

**The candidate receipt lists need the path parameterised.**
`private-candidate-card149/consumers.ts` resolves
`../acowtancy/bovine-accelerator-desktop`. Card 149 is live and
operator-held. Read the consumer set from a gitignored
`config/private-consumers.json`, defaulting to the public consumers when it is
absent, so the receipt can still be produced publicly with a smaller consumer
set and privately with the full one. Card 127's copy is superseded and can
move to the overlay with the rest.

**The proof shape is renamed, not anonymised.** `bovine` is a proof consumer
shape — "a small split composition", a content workspace with minimal
config/settings and no forced layout host. Rename it `split-shell`, describing
the shape rather than the product, exactly as `greenfield-compositions`
already names `minimal`, `workspace`, `full-hosting` and `optional-server`.
That covers `examples/*/bovine/`, the shape tables in the proofs, and the
contract 012 acceptance bullet.

**Logs and cards are renamed consistently.** A log recording work on the
`split-shell` shape is not falsified by the shape having a descriptive name;
it was never a claim about a company. Do not add a note explaining the
substitution — a note that says "this used to be called X" discloses X.

## Steps

1. Create the `private/` overlay, gitignore it, and move the two dormant
   verifiers and their fixtures into it.
2. Parameterise the Card 149 consumer set behind
   `config/private-consumers.json`; move Card 127's copy to the overlay.
3. Rename `bovine` to `split-shell` across examples, scripts, fixtures, docs
   and crate tests. Remove every `acowtancy` path and the
   `com.acowtancy.bovine-accelerator` identifier.
4. Replace `/Users/tom` in the 18 example and prototype files with a
   placeholder or a resolved-at-runtime path.
5. Neutralise the `audio.infiniteloop.*` test identifiers.
6. Apply the same three tokens to Poodle's 7 files.
7. Re-run `effigy qa` in both repositories.
8. Scan the result: no `acowtancy`, `bovine`, `infiniteloop` or `/Users/tom`
   outside the gitignored overlay.

## History Is Not Rewritten

The 487 commits stay as they are. Rewriting history would invalidate every
existing clone and tag for a repository that is about to be tagged, and the
material is client *names* in documentation rather than credentials.

If the names must not survive in history either, that is a different and much
larger decision — a fresh public repository seeded from a squashed tree — and
it should be taken deliberately rather than folded into this card.

**This card makes the current tree publishable. It does not make the history
publishable.** Whoever flips visibility needs to accept that distinction or
choose the fresh-repository route.

## Acceptance Criteria

- no `acowtancy`, `bovine`, `bovine-accelerator` or `com.acowtancy.*` in the
  working tree of either repository, outside a gitignored overlay
- no `/Users/tom` and no `infiniteloop` identifier
- `effigy qa` green in both repositories
- Card 149's receipt can still be produced, with the public consumer set
- the `private/` overlay is documented as existing, without naming what is in
  it

## Notes

The scrub is 146 files rather than the ~560 an all-names pseudonymisation
would have taken, because the operator's own products stay named. It is also
almost entirely documentation: the published TypeScript source has zero
occurrences of any consumer name, and the eleven crate-source hits are all
inside `#[cfg(test)]` modules.
