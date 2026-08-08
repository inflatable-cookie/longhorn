# 165 Artifact Proof Selection Model

Status: complete — landed 2026-08-09
Owner: Tom
Roadmap: g02.013 batch 2
Governing refs: contract 012 (Acceptance); Card 164
Depends on: Card 164 complete
Auto-start next card: no

## Objective

Rewrite the ten artifact proofs that still encode Longhorn's eighteen-package
selection model, so `proof:artifacts` can be green again.

## Why This Is Not A Repoint

The proofs were assumed to be failing on one thing: a stale Poodle evidence
pin at `../poodle/.artifacts/g12.016-A698XB/`. That was fixed — see below —
and it moved the count from 1 passing to 2. The other ten fail for a second,
larger reason that the fix exposed.

Each proof packs Longhorn's packages by directory and asserts which ones a
consumer shape resolves:

```ts
const longhornPackages = [
  ["@inflatable-cookie/longhorn-core", "core"],
  ["@inflatable-cookie/longhorn-layout", "layout"],
  ...
];
const shapes = {
  bovine: {
    longhorn: ["…/longhorn-core", "…/longhorn-svelte"],
    forbidden: ["…/longhorn-layout", "…/longhorn-surfaces", …],
  },
};
```

Those directories no longer exist, so the proofs fail at `bun pm pack` with an
ENOENT on the working directory. Measured across the ten files: **59
package-directory mappings and 38 forbidden-list sites.**

## The Real Decision

`forbidden` asserts install-absence — that a consumer's isolated root does not
resolve a package it did not select. Card 164 made that unexpressible for the
TypeScript tier: Surfaces, history and layout ship inside
`@inflatable-cookie/longhorn` whether a consumer composes them or not, and
tree-shaking is what keeps them out of a bundle.

This exact decision was already taken once, for nucleus's
`verify-longhorn-consumer-boundary.ts`, and should be applied consistently
here rather than re-litigated per file:

- install-absence for a TypeScript domain becomes **import-absence**, checked
  against the proof consumer's source
- the packed-artifact graph gains the stronger assertion in its place —
  exactly three Longhorn packages resolve and no fourth leaked in
- **Rust crate selection is untouched.** That split is real and measured:
  finch compiles 8 crates of 41, nucleus and loophole 24.

Contract 012's Acceptance list needs amending in the same pass. Three of its
bullets are stated in install terms that the TypeScript tier can no longer
satisfy:

- "Nucleus can install window/layout support without Surfaces"
- "Bovine can install configuration and shell support without layout hosting"
- "a Surface-free root import does not resolve Surface or Surface-transfer
  code"

The third is already the right shape — resolution, not installation. The
first two need the same treatment.

## Scope

| Proof | package dirs | forbidden sites |
| --- | --- | --- |
| `verify-greenfield-card125.ts` | 21 | 13 |
| `verify-app-shell-proof.ts` | 9 | 5 |
| `verify-history-tree-artifacts.ts` | 6 | 4 |
| `verify-history-system-artifacts.ts` | 5 | 4 |
| `operation-notification-artifact-proof/artifacts.ts` | 5 | 0 |
| `verify-native-content-artifacts.ts` | 4 | 8 |
| `settings-composition-proof/artifacts.ts` | 3 | 0 |
| `command-system-artifact-proof/artifacts.ts` | 3 | 0 |
| `bridge-topology-artifact-proof/artifacts.ts` | 3 | 1 |
| `verify-guides-card126.ts` | 0 | 3 |

The tables are uniform, but each proof carries bespoke assertion logic
downstream of its table — `settings-composition-proof` inspects the exact
dependency array of the settings package, for instance — so this is per-file
work, not one rewrite applied ten times.

## Already Done, In Card 164's Follow-up

The Poodle evidence pin is fixed and does not need repeating:

- `scripts/poodle-evidence.ts` derives the evidence path from the root
  manifest's `@inflatable-cookie/poodle-core` pin, so the packs Longhorn
  installs and the packs the proofs verify cannot disagree
- the frozen `39f08c04…` set-id literal is gone from all eight scripts that
  carried it; the set id is now the membership hash the evidence records
- integrity is unchanged: every pack must match its recorded SHA-256, and the
  membership hash must match the recorded set id
- `verify-poodle-preview.ts` passes again

## What Landed

All twelve artifact proofs pass, against one before Card 164 and two after
the evidence pin was fixed.

`scripts/consumer-absence.ts` is the decision made once and applied
everywhere. `splitForbidden` divides a forbidden list into packages, which
keep install-absence, and subpaths, which become import-absence against the
staged consumer's source. It throws rather than degrade if the two are
confused, because the failure mode this replaces was a check that passed
vacuously — `node_modules/@inflatable-cookie/longhorn/layout` can never exist,
so the old assertion was reporting success for a claim it no longer tested.

Where a shape table duplicated what its example consumer imports, the table is
now derived from the consumer. Four proofs had drifted from their own
consumers before this; deriving removes the class.

Three findings worth keeping:

- **The native-content Poodle-edge scan was over-broad.** It read a whole
  artifact for `@inflatable-cookie/poodle-`, which flagged every unrelated
  projection once the tier became one package. Scoped to the native-content
  subtree, which is what the claim always meant.
- **The settings root boundary got stronger.** It allowed exactly one
  dependency, on `longhorn-core`. The consolidated framework package is
  asserted to have no dependencies and no peers at all.
- **Several scope lookups still read `node_modules/@longhorn`,** retired two
  renames ago and invisible because the proofs failed earlier for other
  reasons.

Card 125's greenfield receipt was regenerated rather than relaxed: both
repositories' source commits, all three artifact set ids, the package
inventories, and the per-shape package lists. The Rust set moved too, because
the bindings crate and two crate READMEs are inside it.

## Acceptance Criteria

- [x] `proof:artifacts` green — all twelve
- [x] no proof asserts install-absence for a TypeScript domain
- [x] every proof asserts the three-package artifact graph
- [x] Rust crate selection assertions unchanged
- [x] contract 012's Acceptance bullets restated in resolution terms

## Notes

The eleven-of-twelve failure predates Card 164 — it was measured against a
`HEAD` worktree before that card landed. What Card 164 changed is the reason:
it was one stale path, and now it is a selection model that no longer
describes the packages.
