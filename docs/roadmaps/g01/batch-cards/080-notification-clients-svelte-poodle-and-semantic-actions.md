# 080 Notification Clients, Svelte, Poodle, And Semantic Actions

Status: complete
Owner: Tom
Roadmap: g01.012 batch 3
Governing refs: contracts 001, 006, 010, 012, 013, and 016; research memo 016
Depends on: Card 079
Auto-start next card: no

## Objective

Generate the checked notification protocol, narrow host assembly, isolated
Svelte sessions, and public-Poodle retained-panel and transient-toast
projections with fresh semantic-action admission.

## Scope

- Rust-generated notification protocol and fixtures
- framework-neutral `@longhorn/notifications` client
- `longhorn-tauri-notifications` and optional `/tauri` composition
- listener-first snapshot/event reconciliation
- `@longhorn/notifications/svelte` per-instance state
- `@longhorn/notifications/poodle` panel and toast mapping
- public `ToastHost` and `ToastStack` timer-policy composition
- explicit mark-seen and dismiss commands
- injected semantic-action resolver with fresh command admission

## Out Of Scope

- native OS notifications
- product wording, redaction, or localization policy
- durable executable action closures
- operation package required by the notification root
- Poodle source fork
- donor repository writes

## Steps

1. Define and generate the strict notification wire protocol.
2. Add direct and serialized checked clients.
3. Add injected Tauri handlers, events, and narrow capabilities.
4. Implement listener-first Svelte paging and unseen state.
5. Map selected new records into Poodle transient toasts.
6. Map the retained ledger into a controlled Poodle panel.
7. Keep toast expiry separate from mark-seen and dismissal.
8. Resolve action references through an injected fresh-admission executor.
9. Prove remount, multi-window, stale command, and optional-edge behavior.

## Acceptance Criteria

- Rust and TypeScript wire fixtures match exactly
- direct, serialized, and Tauri traces converge
- gaps and epoch changes force refresh
- two Svelte instances do not share renderer authority
- toast expiry removes only transient presentation
- retained panel state survives toast expiry and renderer remount
- action invocation performs fresh consumer admission
- Poodle integration uses public `ToastHost` and `ToastStack`
- framework-neutral root imports no operation, command, Tauri, Svelte, or Poodle

## Evidence Required

- generated fixture diff
- direct/serialized/Tauri trace matrix
- listener-first, gap, stale-event, and remount fixtures
- toast expiry versus retained-ledger fixture
- seen/dismiss multi-window fixture
- semantic-action stale-admission fixture
- Poodle public-import and timer-policy audit
- package export, optional-peer, capability, and dependency audits
- focused Rust, TypeScript, Svelte, clippy, docs, formatting, and Effigy checks

## Stop Conditions

- Poodle timer behavior forces retained-ledger mutation
- semantic actions require unchecked command strings
- multi-window state requires a renderer global
- native OS delivery policy enters the v1 client
- notification root gains a mandatory operation or command dependency

## Next Task

Card 081 is ready after client and presentation proof. Pack isolated consumer
graphs, compare native/renderer traces, document composition, and close
g01.012.
