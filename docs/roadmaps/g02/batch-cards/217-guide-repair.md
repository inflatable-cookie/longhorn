# 217 Guide Repair

Status: ready
Owner: Tom
Roadmap: g02.025 batch 3
Governing refs: contract 012; memo 023 (docs-H3, M3, M4)
Depends on: Card 215 (the guides quote the model it fixes)
Auto-start next card: no

## Objective

A fresh reader following the adoption guides reaches a resolvable manifest
against the tree the guides ship with.

## Why this exists

- `docs/guides/package-selection.md`: the "Add One System At A Time" table is
  duplicated verbatim-ish (`:79-95`, `:112-128` — a merge artifact); both
  copies select the deleted `longhorn-layout`; the copy-paste manifest
  (`:50-59`) lists `"@inflatable-cookie/longhorn/config": "0.1.0"` as a
  dependency key — subpaths are not installable packages. A consumer
  following this guide cannot install.
- `docs/guides/getting-started.md:95-101` installs the 17-tarball layout
  frozen in the Card 127 receipt; the tree produces 3 tarballs. The guide's
  escape hatch points at the same frozen receipt. No working TypeScript
  install path is documented. (The Rust half is accurate.)
- `docs/guides/system-composition.md` teaches the removed hierarchy
  (`:55,58,64`).
- `prototypes/gpui-windowing/README.md:9` describes `docs:rust` as part of
  every qa lane — false.

## Scope

- the three guides and one prototype README named above
- the install recipe verified against the actual tree (bun, `file:` refs,
  the overrides pair the papercuts document)

## Steps

1. `package-selection.md`: deduplicate the table; replace deleted-crate
   selections with the post-179 crates; rewrite the manifest against
   `@inflatable-cookie/longhorn` with subpath imports; verify the recipe by
  following it in a scratch consumer.
2. `getting-started.md`: write the TypeScript install path against the
   three-package tree — `file:` deps plus the `overrides` pair, the
   `effigy deps link bun` development path, and the published-version path
   named as arriving with g02.014. Keep the Card 127 receipt reference as
   history.
3. `system-composition.md`: the Surface-as-layout hierarchy, per Card 215.
4. The gpui README's `docs:rust` claim corrected (and Card 219 decides
   whether the claim becomes true instead).
5. Run `verify-guides-card126.ts` and `verify-documented-commands.ts`; every
   command the guides name must pass the verifiers.

## Do Not

- Document the published install path as if it exists. It does not until
  g02.014; the guide says so and gives the `file:` path that works today.

## Acceptance Criteria

- [ ] the manifest in `package-selection.md` resolves in a scratch consumer
- [ ] `getting-started.md`'s TS path works when followed literally
- [ ] no guide teaches the removed hierarchy
- [ ] the guide verifiers pass

## Evidence Required

- the scratch-consumer run
- the verifier outputs

## Stop Conditions

Stop if the scratch consumer needs a Poodle version that is not published —
that crosses into the g02.014 / Poodle v0.2.0 dependency and the guide says
so instead of pretending.
