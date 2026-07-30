# Restore, Conflict, And Recovery Settings Flow

Date: 2026-07-29
Card: g01.008 / 047
Status: complete

## Outcome

Exposed the existing contract-004 restore engine through checked Rust
projections, generated TypeScript, a narrow Tauri authority, optional settings
registration, and one public-Poodle destructive flow.

The shell owns presentation and explicit user choices. It does not own archive
selection paths, unlock material, compatibility rules, planning, staging,
safety backup, publication, rollback, adapters, journals, or recovery.

## State Machine

| Phase | Renderer input | Host evidence and authority | Result |
| --- | --- | --- | --- |
| inspect | inventory digest or host-picker request | exact bytes, unlock, integrity, authenticity, identity, compatibility | locked, rejected, or complete inspection |
| plan | generation, archive digest, one choice per included domain | fresh current bytes, derived action, migration preparation | rejection or confirmation-bound plan |
| execute | generation and plan digest | complete private staging, fresh evidence, safety backup, durable journal | success, verified rollback, or recovery required |
| adapter | archive/domain/adapter confirmation and minimum guarantee | fresh adapter identity and adapter-owned transaction | separate exact adapter receipt |
| recover | request identity | durable journal, rollback material, live evidence | recovered or recovery still required |

Closing the settings view after execution starts has no cancellation meaning.
The promise and operation remain host-owned. The page registers no ambient
listeners.

## Terminal Receipt Matrix

| Terminal | Ordinary mutation gate | Evidence |
| --- | --- | --- |
| succeeded | open after fresh snapshot | staging counts, safety backup, restored/deleted/migrated/unchanged/skipped/excluded domains |
| rolled back | open after verified rollback | failed stage, domain, rollback terminal, fresh snapshot |
| adapter completed | follows adapter receipt | adapter, participation, digest, semantic evidence |
| recovery required | closed | failed stage, safety backup digest when readable, fresh gated snapshot |
| recovered | open only after verified terminal state | recovery outcome and considered domains |

`ConfigStore` remains the mutation gate. `longhorn-settings-config` maps
restore-active and restore-recovery-required to distinct recovery states; the
renderer cannot dismiss either state into writable settings.

## Fixture Matrix

- ready inspection separates byte integrity, independent authenticity,
  application identity, producer identity, consistency groups, included
  domains, migrations, adapter participation, and exclusions
- locked inspection retains unlock material in host authority
- corrupt and future archives reject with separate typed codes
- every included ordinary or blocked domain requires an explicit archive or
  current-state choice
- stale planning rejects with `restorePlanStale` and publishes nothing
- execution fixtures cover success, verified rollback, and recovery required
- custom SQLite-style participation uses a separate confirmation and receipt
- secret domains appear only as exclusion metadata; no secret payload shape
  exists on the wire

## Authority Audit

- renderer commands contain no filesystem path, archive bytes, passphrase,
  identity secret, executable plan, journal path, or rollback bytes
- host selection resolves inventory digests or invokes an injected picker
- unlock returns only safe state across the port; concrete bytes remain inside
  the operation authority
- planning and execution retain exact generation and digest binding
- Tauri read permission admits snapshot and inspection only
- conflict planning, restore execution, adapter execution, and recovery use a
  separate destructive permission
- Tauri capabilities remain an outer allow-list; the injected authority still
  authorizes each window

## UI And Accessibility

- archive selection is labelled and inspection is non-mutating
- domain decisions use named Poodle radio groups with no automatic choice
- non-restorable domains cannot select archive state
- the exact plan shows actions, current evidence, counts, and confirmation
  digest before destructive confirmation
- adapter execution has a separate confirmation and result
- active and recovery-required states replace ordinary restore controls
- assertive callouts announce publication and recovery
- progress copy states that closing the view does not cancel host work
- all components come from public `@poodle/svelte` exports

## Structure

The expanded Rust protocol, projections, binding fixtures, TypeScript
compatibility checks, and generated declarations were split along
storage/backup/restore boundaries. Card 047 adds no new high-severity doctor
finding. The remaining high-severity God-file finding predates this card.

## Evidence

- Rust restore mechanism tests retain staging, stale-evidence, safety-backup,
  rollback, crash, and recovery guarantees
- Rust projection fixture covers every command and terminal family
- TypeScript guards validate every generated restore payload and reject
  unknown variants or malformed digests
- serialized client and direct Tauri handler tests cover all restore commands
- settings registration and recovery mapping tests prove independent admission
  and the host mutation gate
- mounted Poodle tests cover explicit conflict choices, stale planning,
  locked/corrupt/future archives, adapter separation, recovery gating, and
  host-owned teardown
- package, capability, dependency, payload, secret, authority, and public
  Poodle audits remain in the focused Effigy lane

## Limits

- consumers still assemble concrete authorization, idempotency, picker,
  unlock, archive custody, restore state, and adapter authorities
- generic merge UI and product repair tools remain out of scope
- Card 048 owns artifact-installed three-shape composition and milestone
  closeout
