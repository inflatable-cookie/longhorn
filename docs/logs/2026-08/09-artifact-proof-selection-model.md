# Artifact Proof Selection Model

Date: 2026-08-09
Card: 165
Roadmap: g02.013

## Result

`proof:artifacts` is green across all twelve proofs. It had been one of twelve
since before Card 164 landed.

## The Diagnosis Was Wrong Twice

The lane was blamed on a stale Poodle evidence pin at
`.artifacts/g12.016-A698XB/`. Fixing it moved the count from one to two, and
that was recorded as the whole story. It was not.

Eight scripts hardcoded both the evidence path and the expected artifact set
id. Poodle mints a fresh `svelte-pack-install-*` directory on every pack
rebuild, so all eight rotted together. `scripts/poodle-evidence.ts` now derives
the path from the root manifest's `poodle-core` pin — the one place that
already has to be correct, because if the packs Longhorn installs and the packs
the proofs verify could disagree, the proofs would be verifying something no
consumer resolves. The integrity claim is unchanged: every pack must match its
recorded SHA-256, and the set id must equal the membership hash.

What that fix exposed was the real blocker. Each proof packs Longhorn's
packages *by directory* and asserts which ones each consumer shape resolves, so
all ten still encoded the eighteen-package selection model and died at
`bun pm pack` with an ENOENT on a directory that no longer exists.

## The Claim That Had To Change

`forbidden` asserted install-absence: no
`node_modules/@inflatable-cookie/longhorn-layout`. Layout now ships inside
`@inflatable-cookie/longhorn` whether a consumer composes it or not, so that
directory can never exist — and the old assertion would pass **vacuously**.
That is worse than failing, because it reads as evidence.

`scripts/consumer-absence.ts` splits it: packages keep install-absence,
subpaths become import-absence against the staged consumer's source, and
`splitForbidden` throws rather than silently degrade if the two are confused.
Nucleus's boundary verifier made this call first during Card 164; the proofs
now match it rather than re-litigating it ten times.

Contract 012's Acceptance list carried the same problem in prose. Two bullets
were stated in install terms the TypeScript tier can no longer satisfy, and are
now stated as imports. The Rust tier keeps the install claim, where the split
is real and measured.

## Derivation Beats Duplication

Four proofs' shape tables had drifted from the consumers they describe — they
listed packages the example consumer no longer imported, or missed ones it did.
Where a table duplicated what its consumer imports, it is now derived from the
consumer at run time, which removes the class rather than fixing four
instances.

## Three Things Found On The Way

- **The native-content Poodle-edge scan was over-broad.** It read a whole
  artifact for `@inflatable-cookie/poodle-`, which flags every unrelated
  projection once the tier is one package. Scoped to the native-content
  subtree, which is what the claim always meant.
- **The settings root boundary got stronger.** It allowed the settings package
  exactly one dependency, on `longhorn-core`. The consolidated framework
  package is asserted to have no dependencies and no peers at all.
- **Several scope lookups still read `node_modules/@longhorn`**, retired two
  renames ago and invisible because the proofs were failing earlier for other
  reasons.

## Receipt Regeneration

Card 125's greenfield receipt freezes both repositories' source commits, three
artifact set ids, the package inventories, and the per-shape package lists.
Every one of them moved, so it was regenerated rather than relaxed. The Rust
set moved too — the bindings crate and two crate READMEs are inside it, and
both changed when the generator was repointed at the consolidated layout.

## Remaining

Card 149's candidate receipt still needs regenerating; it stays operator-held
on consumer manifest quiescence. The frozen migration fixtures under
`fixtures/migration/` and the verifiers that assert against them keep the old
eighteen names deliberately — they record what consumers looked like on those
cards' dates.
