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

`proof:artifacts`, a member of `qa`, runs thirteen of them in order:

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
| `verify-documented-commands.ts` | every command the examples' READMEs name exists |

The rest are wired individually: `check:api-reference` runs
`generate-api-reference-card126.ts`, and `verify-held-surface.ts`,
`verify-host-protocol.ts` and `verify-private-candidate-docs-card127.ts` are
each their own selector. `check:agent-control-skill` runs
`verify-agent-control-skill.ts` (skill-vs-vocabulary drift, finder fixtures,
install fixtures). `agent-control:install-skill` copies
`skills/agent-control/` into a consumer repo
(`effigy agent-control:install-skill -- <git-repo>` or
`effigy agent-control:install-skill -- --repo <git-repo>`); the bun script
remains the implementation. `bootstrap:deps` runs the locked Bun install fresh
worktrees need before TypeScript checks; `check:bun-deps` guards
`check:ts`/`check:svelte` when run alone. `consumer-absence.ts` and
`poodle-release.ts` are shared modules, not entry points.

## No consumer knowledge

Longhorn keeps no code with direct knowledge of a consuming application. No
script resolves a sibling repository or reads a consumer's manifest, and only
`verify-greenfield-card125.ts` takes a `*_REPO` environment override
(`POODLE_REPO`, for the admitted Poodle-pack escape). Twenty-three verifiers
that did were deleted on 2026-08-10, along with the frozen receipts they
asserted against; both remain in git history.

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

`check:repo-containment`, also in `qa`, is stricter about location: Cargo
`path`, package `file:` / `link:`, and `join(repoRoot, "../…")` must resolve
inside this tree. The only admitted escape is
`scripts/verify-greenfield-card125.ts`, which packs Poodle from source under
the release workflow. That script reads `POODLE_REPO` to locate the Poodle
checkout (defaulting to the `../poodle` sibling); no other script takes a
`*_REPO` override.

`verify-greenfield-card125.ts` separately enforces the absence of donor
vocabulary inside the greenfield example root.

## CI rehearsal

`effigy ci:rehearse` reproduces the four developer-machine properties that hid
defects from local `qa` during the first release runs:

1. `check:repo-containment` + `check:consumer-isolation` — no sibling path
2. `check:runner-tools` — scripts must not invoke `rg`
3. `cargo fetch --locked` under a fresh `CARGO_HOME`, then offline metadata
4. `CI=1 effigy proof:artifacts` under that `CARGO_HOME` — coloured vitest output
   and a cold cache

Not a substitute for dispatching `release.yml`. Run it before a release when
`qa` is green and you want the clean-runner reading, not the developer one.

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

When bumping `LONGHORN_GENERAL_MSRV`, run `effigy release:floor` in the same
change before commit. The floor gate is what unlocks MSRV-gated Clippy lints;
do not leave that debt for release prep.
