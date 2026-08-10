# Secondary-consumer Behavior, Authority, And Rollback Freeze

Date: 2026-08-02
Roadmap: g01.016
Card: 113
State: complete; Card 114 ready

## Result

The read-only common freeze passed for Soundcheck, soundcheck-library, Signal,
Split Shell Desktop, Jetstream, Poodle, and the compiled Longhorn
graph. No consumer or Poodle file changed.

## Evidence

- exact branches and commits for six external repositories
- clean Soundcheck, soundcheck-library, Signal, Jetstream, and Poodle trees
- exact 19-path Split-shell overlap, all below `docs/` or `CHANGELOG.md`
- exact app ids and active Northstar lanes
- 23 source-backed behavior and toolchain checks
- 27 unique selected Longhorn package paths
- zero selected/forbidden package overlap across all three consumer graphs
- prior exact Poodle artifact-set receipt retained for Card 114 comparison

The fixture is
the migration receipt (retired 2026-08-10; in git history).
The verifier was removed 2026-08-10 — Longhorn no longer keeps consumer-aware
proofs — so the fixture above is the retained evidence.

## Authority

Soundcheck keeps SQLite, backup semantics, scan execution, product settings,
DAW and inspection policy; soundcheck-library and Signal retain their named
sibling domains. Split-shell keeps repository, content, navigation, editorial,
Git, and local SplitView meaning. Jetstream keeps product command execution,
snapshot payload, WGPU/native-view storage, rendering, world, and semantic
input.

## Rollback And Release Boundary

Every later slice requires retained source, exact receipts, and previous
locks. Dual writes, silent fallback, and automatic cleanup are false.
Package-manager publication, tags, and hosted releases remain false.

## Validation

```sh
effigy qa:northstar:g01-secondary-consumer-card113
```

## Next

Execute Card 114. Produce the exact private Longhorn and Poodle artifacts and
install the three selected graphs in isolated consumer-shaped proofs. Keep all
consumer repositories read-only and publish nothing.
