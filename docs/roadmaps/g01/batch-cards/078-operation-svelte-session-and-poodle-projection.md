# 078 Operation Svelte Session And Poodle Projection

Status: complete
Owner: Tom
Roadmap: g01.012 batch 2
Governing refs: contracts 001, 012, 013, and 015; research memo 016
Depends on: Card 077
Auto-start next card: no

## Objective

Add isolated operation sessions and controlled Poodle projections that fit
Soundcheck scan and Loophole render without moving durable truth or product
copy into the renderer.

## Scope

- `@inflatable-cookie/longhorn-operation/svelte`
- per-instance connection, snapshot, active/recent, and pending-command state
- exact mount, remount, and teardown behavior
- request-keyed cancellation and dismissal state
- `@inflatable-cookie/longhorn-operation/poodle`
- public `Progress`, `StatusIndicator`, list, button, and dialog composition
- Soundcheck scan and Loophole queue presentation fixtures
- no-Svelte and no-Poodle import proofs

## Out Of Scope

- retained notifications or toasts
- product phase panels, warning lists, reports, logs, and artifacts
- Poodle source changes unless a separately recorded public seam is missing
- cross-window shared renderer globals
- donor repository writes

## Steps

1. Implement per-instance listener-first Svelte sessions.
2. Add current, recent, selected, and pending-command projections.
3. Make mounted teardown exact and host-work-neutral.
4. Map determinate and indeterminate progress to public Poodle primitives.
5. Map lifecycle and cancellation to controlled status/actions.
6. Mount Soundcheck and Loophole presentation fixtures over one controller.
7. Prove remount, stale result, multi-window, and failure behavior.
8. Audit Poodle public usage and optional peer boundaries.

## Acceptance Criteria

- two mounted instances do not share renderer state
- unmount removes listeners exactly and never cancels host work
- remount restores current host truth
- stale cancellation and dismissal results do not overwrite newer snapshots
- both donor shapes use one controller
- product labels and detail panes remain injected
- Poodle integration uses only public feedback primitives
- framework-neutral and Svelte-only imports resolve without Poodle

## Evidence Required

- mounted lifecycle and listener-count fixtures
- stale-result and authority-epoch traces
- multi-window observation fixture
- Soundcheck and Loophole controlled projection fixtures
- Poodle public-import audit
- package export, optional-peer, and dependency audit
- focused TypeScript, Svelte, docs, formatting, and Effigy checks

## Stop Conditions

- both donor shapes require incompatible controller state
- Poodle public progress/status primitives cannot represent the contract
- product payload must enter shared Svelte state
- component unmount must cancel host execution
- a renderer global is required for multi-window truth

## Next Task

Card 079 is ready after the operation presentation checkpoint. Implement the
independent retained notification ledger and optional operation observer.

## Completion Evidence

- `OperationController` owns listener-first connection, monotonic snapshot
  installation, selection, and request-keyed cancellation and dismissal.
- `OperationSession` mirrors one controller per renderer instance. Late
  listener registration, unmount, immediate remount, and shared-authority
  multi-window observation leave no renderer global or host cancellation.
- Delayed cancellation and dismissal results cannot replace a newer catalogue
  revision or authority epoch.
- One controlled `OperationPanel` projects Soundcheck scan and Loophole queue
  fixtures through public Poodle progress, status, list, button, and dialog
  primitives.
- Product labels enter through bounded authority projections. Product detail
  remains an injected snippet.
- Root, Svelte, Poodle-public-import, SSR, optional-peer, and dry-pack package
  audits pass.
