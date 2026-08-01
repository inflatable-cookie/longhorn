# Linear History Artifact Proof And Checkpoint

Date: 2026-07-30
Card: 067
Roadmap: g01.011

## Result

Clean minimal and Loophole-shaped consumers run from produced TypeScript and
private Rust inventory artifacts. Native and renderer traces match. The
minimal graph has no optional desktop or UI edge. The rich graph retains the
claimed linear Loophole mechanics without moving payload, product apply,
journal, storage, or authorization into Longhorn.

The public linear slice is complete. g01.011 pauses before private fork work.

## Artifact Evidence

TypeScript:

| Package | Version | SHA-256 |
| --- | --- | --- |
| `@longhorn/core` | 0.1.0 | `b41c3585e48f8e538acaccc68209660e7be55962bdea0af443af924011fcc9f0` |
| `@longhorn/history` | 0.1.0 | `a73813a2c43bdcb45934c85bbe15c447db5af92e8abb1a3cc010dfb0aae73333` |

Exact Poodle artifact set:
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`.

Private Rust inventory archives:

| Package | SHA-256 |
| --- | --- |
| `longhorn-core` | `d11aef2cf0e3deb87ec602750522d7cdcc29a5ba6f8c8b4badc7275236d0f210` |
| `longhorn-history` | `acbe9977915042a59ab746bb999ef457c004badc5b1e9da323e1d645a3497292` |
| `longhorn-tauri-history` | `faac81347c7f09c1776763a03902404b7f9a5ab72a72deb7c520ed4f869f81f3` |

Each crate passes `cargo +1.85.0 package --list --allow-dirty`, unpacks into a
clean temporary workspace, inherits the repository's compatible lock
selection, and runs offline under Rust 1.85. These are private source
inventories, not registry-normalized crates.

## Dependency And Capability Matrix

| Shape | Rust | TypeScript | Permissions |
| --- | --- | --- | --- |
| Minimal | core, history | core, history root | none |
| Loophole | core, history, Tauri history | core, history root plus `/tauri`, `/svelte`, `/poodle` | history read/mutate, listen/unlisten |

Minimal resolves no config, bridge, Tauri TypeScript, Svelte, Poodle, or
journal package. Loophole resolves one Svelte 5.38.6 runtime and the exact
Poodle preview artifacts. Neither lock contains a workspace alias or sibling
source path.

## Semantic Evidence

| Area | Result |
| --- | --- |
| minimal | materially different preference payload; record, adjacent coalesce, undo, authoritative future |
| Loophole record | typed rename coalesces; the unwired donor's timed 750 ms grouping capability compounds; count limit advances baseline |
| navigation | undo, redo, and stable-ID checkout use checked product apply |
| native/renderer | each renderer fixture comes from its Rust run; final summary and full past/current/future page match |
| reload | strict snapshot reload preserves structural history |
| journal | one durable suffix replays with exact transition equality |
| cross-session | recovered history can undo the durable suffix |
| event gap | changed authority epoch triggers authoritative refresh |
| teardown | direct and Tauri listeners each unlisten once |
| UI | one mounted rich proof uses the public Poodle history panel |

The rich final trace is revision 7 with one applied entry and two
authoritative future entries. The minimal final trace is revision 6 with the
same linear position shape.

## Failure And Durability Evidence

| Failure | Model | History | Result |
| --- | --- | --- | --- |
| stale plan | unchanged | unchanged | rejected before apply |
| apply failure, rollback verified | exact source | exact source | typed rolled-back failure |
| rollback failure | partial mutation reported | exact source | terminal rollback-failed evidence |
| future structural version | untouched | untouched | rejected |
| foreign codec family | untouched | untouched | rejected |
| corrupt payload | untouched | untouched | rejected |
| journal append after in-memory commit | in-memory action committed | revision 3 | durable suffix stays at revision 2; recovery required |

An event hint never claims durability. A Tauri capability never claims product
authorization.

## Boundary Audits

- generated history bindings have zero drift
- renderer fixtures contain no product payload
- shared artifacts contain no Loophole payload or codec marker
- product apply and rollback stay in the rich consumer
- snapshot and journal policy stay in the rich consumer
- minimal optional edges are absent
- rich capability permissions are exact
- Poodle usage stays on public primitives
- no branch, project-version, or durable-event-source API enters artifacts
- no donor repository was written

## Behavior Delta

| Class | Result |
| --- | --- |
| retained | record, coalesce, 750 ms grouping capability, 100-entry default, limit, undo, redo, checkout, snapshot, journal suffix, recovery, cross-session undo, labels |
| improved | plan/apply/commit, verified rollback invariance, terminal partial-model evidence, strict codecs, authoritative future, event-gap refresh, teardown |
| rejected | renderer redo authority, move-before-apply, silent discard, generic payload transport, capability-as-authorization, durable event claim |
| deferred | Loophole cutover, fork tree, branch references, checkpoints, project versions, collaboration, public release |

## Validation

- `effigy proof:history-system-artifacts`
- two isolated TypeScript installs and executable traces
- two offline Rust consumer runs
- one isolated mounted Svelte/Poodle test
- exact dependency, artifact, peer, capability, payload, authority, recovery,
  and durability audits
- full `effigy qa`

One cold full run hit the 20-second settings SSR timeout. Its targeted rerun
passed in 9.1 seconds. The clean full rerun passed the same selector in 6.2
seconds and completed without failures.

## Checkpoint

Card 067 completes the public linear runway. Northstar posture becomes
`strict-paused`. Card 068 is planned, not ready. It may start only after the
user reviews this checkpoint and authorizes private fork evidence work.
