# 125 Greenfield Composition Matrix

Status: complete
Owner: Tom
Roadmap: g01.016 batch 5
Governing refs: contracts 002, 004-007, 009-017; Cards 119, 121, and 124
Depends on: Card 124
Auto-start next card: no

## Objective

Provide four small, buildable, produced-artifact examples that show how a new
Tauri app selects Longhorn capabilities without donor baggage or an umbrella
package.

## Repository Scope

- Longhorn: examples, fixtures, artifact installers, docs, and validation.
- Consumer and Poodle repositories: read-only exact artifact receipts.
- Package registries: no writes.

## Scope

- minimal config/settings shell
- no-Surface workspace window/layout/settings/commands shell
- full window/Surface/region/panel/transfer/settings/commands/linear-history host
- optional-server bridge/supervision composition over local domains
- exact Rust and renderer package manifests, capabilities, startup, teardown,
  storage profile, and failure posture
- produced-artifact installation outside workspace resolution

## Steps

1. Define one explicit capability manifest for each composition.
2. Build minimal Rust and Svelte/Tauri roots from produced artifacts.
3. Prove startup, authoritative first load, visible failure, and teardown.
4. Prove storage paths and one safe config mutation in each applicable shape.
5. Prove optional edges disappear from unselected graphs.
6. Prove optional service failure does not invalidate local domain authority.
7. Record exact commands, files, package inventories, and extension points.
8. Keep product pages, commands, payloads, and policies as injected examples.

## Acceptance Criteria

- every example builds outside Longhorn workspace resolution
- no example resolves an umbrella package
- minimal resolves no layout, Surface, bridge, history, or native-content edge
- workspace resolves no Surface or service edge
- full hosting preserves the complete optional hierarchy with linear history only
- optional-server can remove supervision and remain a valid local app
- one Svelte/Poodle/Tauri runtime resolves per renderer
- examples contain no donor product vocabulary or copied Poodle component

## Stop Conditions

- an example requires an unpublished sibling source to build
- optional packages leak upward into smaller graphs
- a template invents product behavior or authority
- a package-manager publication is proposed as a build prerequisite

## Evidence

- four renderer manifests install packed Longhorn and Poodle artifacts outside
  both source workspaces; Svelte check, Vite build, and mounted lifecycle pass
- four default Rust roots and optional server without supervision run locked,
  offline, and on Rust 1.85 from 26 archived crate inventories
- exact TypeScript/Rust graph counts are 4/6, 8/16, 12/24, and 5/8; optional
  server local mode removes both bridge crates and retains six Rust packages
- every root proves platform-native storage, compiled default, coordinated
  atomic mutation, file reload, visible failure, and teardown
- the checked receipt is
  `../../../../fixtures/greenfield/card125/composition-matrix-v1.json`
- package-manager publication remains false

## Next Task

Execute Card 126. Turn the proven compositions and migrations into the public
API, storage, backup, topology, and upgrade guide set.
