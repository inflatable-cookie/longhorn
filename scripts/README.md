# Scripts

Repo-owned automation that Effigy cannot express generically. Run everything
from the repo root:

```sh
effigy tasks
effigy doctor
effigy qa
```

## What runs where

Every script here is reachable from a selector. Nothing in this directory is
meant to be run by remembering it exists — if something is worth keeping, it is
worth wiring, and if it is not wired it should be deleted.

`proof:artifacts`, a member of `qa`, runs twelve of them in order:

| script | proves |
| --- | --- |
| `verify-app-shell-proof.ts` | the composed shell mounts against both host adapters |
| `verify-bridge-topology-conformance.ts` | each topology example imports only its declared surface |
| `verify-bridge-topology-artifacts.ts` | the five topologies install and run as isolated graphs |
| `verify-settings-composition-proof.ts` | the settings composition example against the real registry |
| `verify-command-system-artifacts.ts` | the command system example, packed and installed |
| `verify-history-system-artifacts.ts` | linear history, packed and installed |
| `verify-history-tree-artifacts.ts` | fork history, packed and installed |
| `verify-operation-notification-artifacts.ts` | operations and notifications together |
| `verify-native-content-artifacts.ts` | native content, including the three Tauri examples |
| `verify-poodle-preview.ts` | the Poodle projection renders under both backends |
| `verify-greenfield-card125.ts` | the greenfield compositions carry no donor vocabulary |
| `verify-guides-card126.ts` | the guides match the generated API surface |

The rest are wired individually: `check:api-reference` runs
`generate-api-reference-card126.ts`, and `verify-held-surface.ts`,
`verify-host-protocol.ts` and `verify-private-candidate-docs-card127.ts` are
each their own selector. `consumer-absence.ts` and `poodle-evidence.ts` are
shared modules, not entry points.

## No consumer knowledge

Longhorn keeps no code with direct knowledge of a consuming application. No
script resolves a sibling repository, reads a consumer's manifest, or takes a
`*_REPO` environment override. Twenty-three verifiers that did were deleted on
2026-08-10, along with the frozen receipts they asserted against; both remain
in git history.

Consumer *names* may still appear as vocabulary — a test graph shaped like a
real application's, an example named for the topology it exercises. What may
not appear is a path, a pinned consumer commit, or an assertion about another
repository's working tree. A proof that reads a second repository can only be
green when both are simultaneously in one state, which is not a property this
repository can hold.

`check:consumer-isolation`, a member of `qa`, enforces this. It names no
consumer — it detects the mechanism, so a consumer arriving or leaving never
requires editing it. Three things fail it:

- a `*_REPO` environment override
- an absolute path into another checkout
- `resolve(repoRoot, "../name")` onto a sibling

Poodle and poodle-specs are admitted by name in `DEPENDENCIES`, because
Longhorn projects into them rather than the other way round. A fourth entry
should be argued for.

`verify-greenfield-card125.ts` separately enforces the absence of donor
vocabulary inside the greenfield example root.

## Escape hatches

- `KEEP_HISTORY_TREE_PROOF=1` retains the fork-history proof's disposable
  artifact workspace.
- `KEEP_GREENFIELD_COMPOSITION_PROOF=1` does the same for the greenfield
  composition proof.

Neither attempts registry publication.

## Language

Prefer Effigy for generic operations. If Longhorn needs repo-owned automation,
use TypeScript with Bun. Bash is thin glue only; Python needs a concrete
technical reason.

## Release gates

- `check-release-floor.sh` — enforces the declared MSRV
  (`release-baselines/rust-toolchains.env`) with Clippy and the full test
  suite at the floor toolchain, not a bare `cargo check`.
- `verify-source-consumer.sh` — builds a throwaway consumer against the
  release commit and asserts every probed longhorn crate resolves from a
  git source, proving the commit is consumable as a tagged dependency.

Run both with `effigy release:gates`.
