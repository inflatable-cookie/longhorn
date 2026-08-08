# Svelte Reactive Client State

Date: 2026-07-29
Card: 037
State: complete

## Outcome

- added private `@inflatable-cookie/longhorn-svelte`
- kept the root free of layout, Surface, transfer, and Poodle imports
- added per-window Svelte 5 rune state over checked clients
- added explicit optional domain subpaths and optional peers
- added consumer-fed layout dispatch without inventing layout IPC
- added request-keyed optimistic layout and Surface projection
- added transfer preparation, lease, completion, and cancellation state
- added mounted, SSR, isolation, teardown, package, and compatibility evidence

## State Transitions

| State | Entry | Exit |
| --- | --- | --- |
| `idle` | construction or completed stop | explicit start |
| `loading` | first supported start | checked snapshot or failure |
| `ready` | accepted authority | reconnect, failure, or stop |
| `reconnecting` | explicit reconnect with retained visible authority | checked newer snapshot or failure |
| `unsupported` | optional capability absent | stop or a new configured instance |
| `failed` | transport, validation, dispatch, or teardown failure | explicit reconnect or stop |

Import creates no state. Each constructor creates one isolated instance.
Stop clears renderer snapshots and projections. Destroy forbids restart.

## Request Reconciliation

| Case | Result |
| --- | --- |
| request begins | one projector is stored under its request id |
| duplicate pending request id | rejected before dispatch |
| authoritative same-or-newer revision | accepted and optimism reapplied |
| older completion after newer document | completion cannot replace authority; its projection clears |
| Surface epoch changes | all prior-epoch optimism clears |
| prior-epoch Surface completion arrives late | ignored even when its revision number is larger |
| committed or rejected response | exact authoritative document reconciles under freshness rules |
| dispatch failure | exact request projection clears; state becomes failed |
| stop or destroy | every pending projection clears |

Layout documents and dispatchers are consumer inputs. Longhorn adds no layout
command, event, persistence, registry, or durable fallback state.

## Mounted Teardown

The package-local mounted fixtures prove:

- two instances retain independent snapshots and status
- rune state updates mounted output
- mount, unmount, and remount call one start and one stop each
- listener registration resolving after unmount unlistens exactly once
- concurrent component cleanup and explicit stop share one teardown promise
- pending panel and Surface preparations cancel when they resolve late
- preparation timers clear
- published leases receive one newer empty replacement
- transfer listeners unlisten once

## Package Boundary

| Entry | Runtime capability |
| --- | --- |
| `@inflatable-cookie/longhorn-svelte` | lifecycle, status, mounted cleanup, generic optimism |
| `@inflatable-cookie/longhorn-svelte/layout` | checked consumer-fed layout projection and dispatch |
| `@inflatable-cookie/longhorn-svelte/surfaces` | Surface snapshot and mutation state |
| `@inflatable-cookie/longhorn-svelte/transfer` | panel preparation, leases, completion, cancellation |
| `@inflatable-cookie/longhorn-svelte/surface-transfer` | optional whole-Surface preparation and completion |

Core is the sole mandatory Longhorn dependency. Layout, Surface, transfer, and
Surface-transfer packages are optional peers. Svelte is a peer. Poodle is not
present.

The peer range is `>=5.38.6 <=5.56.8`. TypeScript, Svelte Check, mounted tests,
SSR tests, and package checks passed at both endpoints. The development lock
remains on 5.38.6 so ordinary QA exercises the floor.

## Validation

- TypeScript passed
- Svelte Check passed without warnings
- 16 client and SSR tests passed
- package dry run contained only 11 declared source files
- dependency and root-import audits passed
- full Effigy QA passed

## Limits

- no Poodle import or visual component
- no layout host endpoint
- no renderer durability
- no consumer migration
- no public registry claim

## Next

Run Card 038 as the named Poodle-local contract and preview-artifact
checkpoint. Card 039 remains blocked until that evidence exists.
