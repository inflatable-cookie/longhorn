# Longhorn 0.2.0 Publication

2026-09-22. Longhorn `0.2.0` is published: three npm packages at `0.2.0`
(`latest`, MIT) and the annotated tag `v0.2.0`. The release carries the staged
update protocol and exclusive admission lease, the packaged `agent-control`
opt-in, and the agent-control stdio carrier.

## Frozen identity

- Tag `v0.2.0` (annotated) dereferences to
  `df09c25a2fd1f6e8cc16c85961c81410f4dfc75e`.
- npm integrity, each verified byte-for-byte against the release run's packed
  artifact:
  - `@inflatable-cookie/longhorn@0.2.0` —
    `sha512-EY/wlIoO8pnxTew/pT6ErtHCm8Edw2qwV5xviGFSh1NtjqjuoIBknux+iEm4NbvF4KbBkYhmGLnLMh8XD5dakA==`
  - `@inflatable-cookie/longhorn-poodle-svelte@0.2.0` —
    `sha512-kPW/9w8DMD/6l/gVcd35Xh5odIOE4uuXIekyXoeegA3oLUqYakRBf3CGDDelgdIK7RHyUVUs2f3fX3ol2/dKRg==`
  - `@inflatable-cookie/longhorn-tauri@0.2.0` —
    `sha512-LG9wTsYCtscKg80aKJ6ELyoedoZPm1AkF+JvcF0Nlgjlg9Aa2I7dkB+Tx2L6/Np5mOzEIVgDh+Ys/Rnfhxxn6g==`
- The Rust crates are `publish = false`. Consumers take them by the git tag —
  there is no registry identity for them.

## What shipped

The four lanes approved for this release, all merged before the cut:

- **Staged update protocol** (`g02.041`, contract 018 amended 2026-09-22):
  prepare/apply, live progress, and the Later/cancel/error lifecycle.
- **Exclusive admission lease** (`g02.042`, contract 018 amended): the lease is
  held through apply, so a second update attempt cannot interleave.
- **Packaged `agent-control` opt-in** (`g02.043`, contract 022 amended): the
  `agent-control` feature with the `evaluate` split. Contract 023's production
  MCP role is withdrawn; the opt-in is the production surface.
- **Agent-control stdio carrier** (`g02.044`): fronts the single contract 022
  instance for harnesses that cannot consume a streamable-HTTP MCP entry.

The `dev` feature keeps `dev = ["agent-control-evaluate"]` as a migration
forward so the consumer link cycle closes on the old name. Remove it at the next
breaking release.

Also in the release: the zero-scope route-commit race fix in
`SettingsSessionDocument.installRoute` (last navigation wins), with its
regression tests.

## Release candidate delta

Desktop acceptance ran against `608a7eed`. The released commit `df09c25a` is a
descendant; after `608a7eed` the only changes were the `0.2.0` version surface
(package versions, the adapter peer on `@inflatable-cookie/longhorn`, the
artifact-proof version constants, the regenerated API reference — all release
metadata) and the release-workflow idempotency guard below. No runtime behaviour
changed between the accepted candidate and the released artifact.

## The partial-publish incident

The first publish run (`35767158100`) failed at the second package. npm returned
a 404 on the publish `PUT` for `@inflatable-cookie/longhorn-poodle-svelte` —
trusted publishing was configured for `@inflatable-cookie/longhorn` only, and
npm answers a trusted-publisher mismatch with 404 rather than 403. Because the
publish loop runs under `set -e`, `@inflatable-cookie/longhorn@0.2.0` landed and
the two adapters never published. Registry propagation then made the state
ambiguous for several minutes: the run log reported the first package published
while the packument still listed only `0.1.0`.

No consumer was affected — the adapters at `0.1.0` peer on
`@inflatable-cookie/longhorn@0.1.0`, which stays on the registry.

The operator fixed the two packages' trusted publishers. The publish step was
then made idempotent (skip a package whose exact version is already on the
registry) and the run was re-dispatched against the tag. A plain re-dispatch
would have died on the already-published first package.

## Evidence

- Release run `35773099043` — green, 38m41s: full QA gate, the seven release
  gates, the tag version check, pack-and-verify, publish.
- The earlier dry-run `35761126145` was green at the same tree.
- Registry integrity equals the packed artifact for all three packages.
- Consumer pre-release acceptance on `608a7eed`: route tests 5/5, upstream route
  regressions 3/3, native updater 15/15, installer 146/146,
  publication/trust 14/14, 23 Cargo links and 3 Bun links resolved.

## Remaining

- The consumer repoint is cross-repo and operator-owned; Longhorn's side is
  done.
- Desktop `g05.179` takes the released artifact; the packaged private A→B proof
  (`g05.180`) follows adoption.
- The `dev` migration forward is temporary. Remove it at the next breaking
  release.
