# Settings Composition Proof And Closeout

Date: 2026-07-29
Card: 048
Roadmap: g01.008

## Result

Four isolated consumers install the settings package family from produced
artifacts and mount one registry/session contract as a modal, window, or
panel. g01.008 is complete.

## Artifact Evidence

The proof produced and installed:

- six private Rust inventory archives: `longhorn-core`, `longhorn-config`,
  `longhorn-settings`, `longhorn-settings-config`,
  `longhorn-tauri-settings`, and `longhorn-tauri-config`
- three npm archives: `@inflatable-cookie/longhorn-core`, `@inflatable-cookie/longhorn-config`, and
  `@inflatable-cookie/longhorn-settings`
- Poodle artifact set
  `39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`

Every TypeScript consumer installed in a clean temporary root. Every resolved
Longhorn package reported version `0.1.0`. Each root resolved Svelte `5.38.6`
once. No workspace link, sibling source alias, duplicate peer runtime, or
unexpected optional Longhorn package was present.

The Rust archives use each crate's `cargo package --list` inventory and build
offline after unpacking into a clean workspace. Public registry-normalized
Cargo packaging is deferred to the release lane because these private
interdependent crates are not published.

## Mounted Matrix

| Shape | Host | Pages | Tests | Optional-boundary result |
| --- | --- | --- | ---: | --- |
| Split-shell | modal | Preferences | 5 | no config, layout, Surface, command, or backend package |
| Soundcheck | window | Audio, Storage, Backups, Restore & Recovery | 4 | config composed explicitly; no layout, Surface, command, or backend |
| Loophole | panel | Application, Appearance, Hardware, Keybindings | 2 | specialist renderers and commands stay consumer-owned |
| Nucleus | window | General | 1 | no Surface or backend navigation; no optional package |

All hosts loaded the sealed registry and scope authority before reveal.
Unmount released the session listeners.

## Behavior Matrix

| Behavior | Evidence |
| --- | --- |
| immediate | Loophole Application publishes one checked command |
| staged | Split-shell and Loophole Appearance publish only on Apply |
| reset | Split-shell sends the separate checked reset command |
| managed policy | Loophole managed control is disabled |
| stale authority | visible conflict; zero publications |
| invalid intent | visible rejection; zero publications |
| policy veto | visible rejection; zero publications |
| activation | remains receipt projection, separate from immediate/staged timing |
| storage | shared page renders authoritative identity and diagnostics |
| backup | inspection is nonmutating; explicit create publishes once |
| restore | inspect and plan are nonmutating; confirmation precedes one execution |
| rollback | `rolledBack` remains a distinct terminal |
| recovery | `recoveryRequired` remains distinct and ordinary mutation is blocked |
| multi-unit apply | visible separate-receipt limit; no atomic publication claim |

## Audits

Dependency:

- root `@inflatable-cookie/longhorn-settings` has no upward optional dependency
- Split-shell resolves only core and settings
- Soundcheck opts into config
- Loophole specialist pages do not pull command or backend packages
- Nucleus proves absence does not create dead navigation

Capability:

- every shape has exact settings read/mutate and event listen/unlisten grants
- Soundcheck alone adds config read and storage/backup/restore mutation
- Loophole hardware and keybinding grants remain consumer-namespaced
- no unused optional-system permission appears

Protocol and authority:

- generated settings and config fixtures drive the proof transports
- settings mutation carries registry generation, scope revision, authority
  token, page, scope, apply unit, and request identity
- config inspection, planning, publication, rollback, and recovery retain
  separate commands and outcomes
- product schemas, renderer callbacks, and specialist command authority remain
  outside Longhorn

Transaction and receipt:

- one apply unit produces one exact receipt
- rejected or unconfirmed work publishes nothing
- restore terminals preserve succeeded, rolled-back, and recovery-required
  meaning
- no cross-domain atomicity is claimed

UI:

- modal, window, and panel use one public-Poodle shell contract
- readiness gates reveal
- failure and recovery states remain visible
- navigation contains only registered pages
- controls are queried by accessible roles and names in mounted proof

## Behavior Delta

| Class | Result |
| --- | --- |
| retained | product schemas, renderers, specialist commands, host choice, managed policy authority |
| changed | registry/session/bootstrap mechanics, checked mutations, shared config operation pages, capability inventory |
| deferred | donor cutover, command/keymap semantics, backend pages, public npm/Cargo release, cross-domain consumer transactions |
| platform-limited | proof is build- and renderer-level; no Windows/Linux packaged runtime claim |

## Validation

- `effigy qa`
- `effigy proof:settings-composition`
- 12 mounted Vitest tests
- four isolated `svelte-check` runs
- six unpacked Rust artifacts checked through one offline consumer
- exact package, capability, Poodle, Svelte, and source-resolution audits
- direct god-file scan: no new high-severity finding; the existing
  `longhorn-tauri-windowing/src/lifecycle/model.rs` finding remains

## Closeout

Cards 042-048 and g01.008 are complete. Northstar returns to a strict paused
post-008 intent checkpoint. No bridge/topology or other milestone starts
without an explicit lane choice.
