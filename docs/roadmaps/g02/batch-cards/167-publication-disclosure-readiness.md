# 167 Publication Disclosure Readiness

Status: complete — landed 2026-08-09
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

**One consumer is client work and must not appear.** Its organisation name,
product name, desktop repository path and reverse-DNS application id are all
removed. The tokens are deliberately not written here — a card that lists what
it scrubbed discloses it, which is the same reason the rename carries no
"formerly known as" note.

Three sibling products of the same client were checked and appear nowhere in
either repository; they exist only in their own private repos.

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

- a developer home directory path in 18 files, all under `examples/` and
  `prototypes/`
- reverse-DNS test identifiers in crate `#[cfg(test)]` modules carrying an
  organisation name that appears nowhere else

No credentials, keys, emails or customer data were found in the tree or in the
history's added-file names.

## No Live Gate Depends On Any Of It

This is what makes the card small. `effigy.toml` references none of the
private-consumer material. That client's two consumer
verifiers are dormant historical scripts belonging to completed cards; they
are in no aggregate, and `proof:artifacts` does not run them.

So nothing that currently gates a release reads a client repository, and
removing that material cannot break a gate.

## What Moves Where

**Third-party evidence leaves the repository.** The two dormant verifiers and
their frozen migration fixtures assert a client's application id, storage leaf
and dependency graph. Move them to a gitignored `private/` overlay with a
committed `private/README.md` stating that consumer-specific migration
evidence for third-party products is held outside the public repository.

Deleting them would be simpler and loses the evidence for two completed cards;
moving keeps it recoverable and keeps the public tree honest about the fact
that it is not the whole record.

**The candidate receipt lists need the path parameterised.**
`private-candidate-card149/consumers.ts` resolved a sibling path into the
client's repository. Card 149 is live and operator-held. Read the consumer set from a gitignored
`config/private-consumers.json`, defaulting to the public consumers when it is
absent, so the receipt can still be produced publicly with a smaller consumer
set and privately with the full one. Card 127's copy is superseded and can
move to the overlay with the rest.

**The proof shape is renamed, not anonymised.** The client also lent its name
to a proof consumer shape — a small split composition, a content workspace
with minimal config and settings and no forced layout host. It becomes
`split-shell`, describing
the shape rather than the product, exactly as `greenfield-compositions`
already names `minimal`, `workspace`, `full-hosting` and `optional-server`.
That covers `examples/*/split-shell/`, the shape tables in the proofs, and the
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
3. Rename the shape to `split-shell` across examples, scripts, fixtures, docs
   and crate tests. Remove every path into the client's repository and the
   reverse-DNS application id.
4. Replace the developer home path in the 18 example and prototype files
   with a neutral placeholder.
5. Neutralise the reverse-DNS test identifiers.
6. Apply the same three tokens to Poodle's 7 files.
7. Re-run `effigy qa` in both repositories.
8. Scan the result: none of the client tokens, no stray organisation
   identifier, and no developer home path outside the gitignored overlay.

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

- none of the client's organisation, product, path or application-id tokens
  in the working tree of either repository, outside a gitignored overlay
- no developer home path and no stray organisation identifier
- `effigy qa` green in both repositories
- Card 149's receipt can still be produced, with the public consumer set
- the `private/` overlay is documented as existing, without naming what is in
  it

## Outcome — 2026-08-09

177 files in Longhorn and 8 in Poodle. The 146 estimate above was measured
from a walk that missed repository-root files and some crate tests; the real
figure is recorded here rather than the estimate being quietly corrected.

Both trees scan clean for every client token, the developer home path and the
stray organisation identifier. `proof:artifacts` is green across all twelve
proofs, alongside `check:ts`, `test:ts`, `check:bindings`,
`check:api-reference`, `held-surface` and the docs gates.

### The card scrubbed itself

The rename pass ran over `docs/`, which includes this card. A document that
names the tokens it is removing cannot survive its own scrub: the decision
section came out reading "that is `private-consumer`, `split-shell`,
`split-shell`", and step 3 as "rename `split-shell` to `split-shell`".

Rewritten to describe the change without naming what was removed, which is
what it should have said in the first place — the same reason the rename
carries no explanatory note. Any future scrub card should be written that way
from the start.

### A blanket rename was the wrong instrument

Replacing the product name across code as well as prose produced
`runSplit-shellTrace`, unquoted `split-shell:` object keys, `.split-shell`
property access and a bare `const split-shell =`. None of that is valid
TypeScript, and four proofs stopped parsing.

All of it was repaired — camelCase identifiers, snake_case in Rust tests,
quoted keys, bracketed access — but the lesson is that prose and identifiers
need separating before the first pass, not after. A hyphenated replacement is
safe in Markdown and in string literals and unsafe everywhere else.

The detector that found the last case is worth keeping: strip quoted strings
from each line, then look for the token in what remains. `bun build` is not a
syntax check, because it resolves imports and fails on missing modules.

### Two threads, one repository

A concurrent thread committed to Longhorn mid-scrub and restored files that
were staged for deletion; only the gitignored overlay survived untouched. The
removals were redone and committed immediately, staging by path so none of the
other thread's work was swept in.

Three agents across two shared repositories will keep colliding. Branches, or
staggering, would remove the class.
