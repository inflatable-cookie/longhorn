# Agent file selection is unreachable under agent control

Status: open — boundary decision needed from the operator
Owner: Longhorn Chatterbox
Next check: when the operator answers the four decisions below

## The gap

When a consumer app opens a file picker, agent control has no path to the
selection. The flow stops and waits for a human. Raised by the operator
2026-09-22.

This is the deferred seam, not a new one. Contract 022 `### Boundaries` says:

> Native menus, native dialogs, and OS-level input are out of scope. Dev builds
> may register mock dialog responders; that seam is app-owned.

Research memo 024 carries it as an open gap:

> Mock dialog responders are app-owned; their seam shape is deferred until a
> consumer needs one.

That consumer need has now arrived, so the deferral is due.

## The contract's stated route does not work

Contract 022 offers `command` as "the route to behavior behind native menus and
dialogs". That assumes dialogs are command-reachable. In practice the picker is a
UI-local call inside a component handler, not a registered command, and where a
consumer has no command registry the route is closed outright. Figmatic recorded
exactly this on 2026-09-15 (`figmatic/PAPERCUTS.md`):

> **Reaching Generalise live requires a native folder dialog** — `ProjectsView`
> creates and imports projects through `@tauri-apps/plugin-dialog`'s `open()`, so
> no project exists in a fresh worktree storage leaf and agent control has no
> path to one (`command` is `Unsupported` in Figmatic).

## What consumers actually use (reconnaissance 2026-09-22)

| Mechanism | Repos | Call sites |
| --- | --- | --- |
| `@tauri-apps/plugin-dialog` `open`/`save` from JS | acowtancy/bovine-desktop, figmatic, finch, nucleus | ~48 |
| Rust-side picker (`pick_file`) | nucleus (`config_operations/restore/commands.rs`) | 1 |
| `<input type="file">` | soundcheck (`LibraryManifestSettings.svelte`) | 1 |

Call-site breakdown: acowtancy ~31, nucleus ~9, figmatic 5, finch 3,
soundcheck 1.

The dominant mechanism is JS `invoke`-backed, and this is the finding that sets
the shape: `open()` returns **paths**, so answering it needs **no byte bridge**.
The app's downstream code is untouched. Only the `<input type="file">` case
needs bytes materialised into real `File` objects.

## Recommendation

Longhorn owns a selection-request seam; the app keeps the policy.

1. **Core crate** (`longhorn-agent-control`) — a pending-selection registry
   (`id`, `kind` = open/save/folder/multi, filters, defaultPath, requesting
   window) with bounded lifetime. Each request is published on the existing push
   surface as a fourth resource beside `console`/`page-error`/`navigation`
   (e.g. `longhorn://agent-control/selection`), so the agent is *told* a
   selection is pending instead of discovering it by timeout.
2. **Two tools** — `answer_selection { id, paths }` and
   `reject_selection { id, reason }`, typed for unknown and expired ids.
3. **Consumer entry point** — a drop-in `open`/`save` replacement in
   `@inflatable-cookie/longhorn` beside the existing agent-control TS shim: when
   an instance is attached, request a selection over the control channel and
   return it; otherwise call `@tauri-apps/plugin-dialog` unchanged. The human
   path stays bit-identical.

Honest framing for the contract amendment: Longhorn still never drives the OS
panel. What changes is that the app's *selection* can be **answered**.

Non-negotiables: no default path and no silent fallback — an unanswered request
stays pending and expires typed, and the app never proceeds on a guess. A picker
already on screen is not retroactively answerable; the design answers before it
opens.

Suggested first slice: `open`/`folder` at the JS boundary (`save` and
`<input type="file">` deferred, and the latter is an explicit non-goal for slice
one).

## Decisions the operator owns

1. **Boundary** — does Longhorn own this seam (contract 022 amendment), or does
   it stay app-owned with each consumer exposing path-taking commands? The
   app-owned route is cheaper per app; figmatic already shows it failing.
2. **Is `save` in the first slice?** A save target is a write reach; `open` and
   `folder` are read.
3. **Packaged or dev-only?** Contract 022 treats `agent-control` as a legitimate
   packaged opt-in and `evaluate` as dev-only. Answering a selection is
   behaviour, closer to `command` than `evaluate`, so packaged parity is
   defensible — but it is the operator's call.
4. **`<input type="file">`** — separate later slice, and take the byte-bridge
   cost up front rather than discovering it mid-implementation?

Also open: which consumer is the acceptance target — acowtancy (most call sites)
or figmatic (the recorded blocker).

## Disposition

Keep open until the operator answers. The answers become a contract 022 amendment
and a `g02` card for the first slice; the reconnaissance is done, so the card can
start at implementation with consumer acceptance named up front.
