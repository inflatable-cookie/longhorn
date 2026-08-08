# Storage Profile Diagnostics And Backup Pages

Date: 2026-07-29
Card: g01.008 / 046
Status: complete

## Outcome

Added checked storage and backup operations from Rust mechanism through Tauri,
TypeScript, settings registration, and public-Poodle pages.

## Protocol Boundary

Renderer state contains:

- active profile, canonical app id, effective leaf, roots, provenance,
  warnings, locator state, and layout digest
- inspected transition domains, unknown source paths, conflicts, evidence
  digest, confirmation digest, and terminal receipts
- valid same-app archives plus preserved locked, corrupt, foreign, unknown,
  unreadable, and unmanaged inventory
- pending-publication status, redacted encryption availability, host-issued
  retention confirmation, and exact publication/deletion receipts

Renderer state excludes:

- filesystem handles or arbitrary browse authority
- portable-root or export target paths supplied as commands
- executable transition, cleanup, or retention plans
- product retention policy
- archive payload bytes
- encryption recipients, identities, or passphrases

Path projection is exact UTF-8 or fails. It never substitutes lossy text for
authority evidence.

## Storage Transition Matrix

| UI state | Renderer sends | Host retains or rechecks | Terminal evidence |
| --- | --- | --- | --- |
| inspect | built-in target profile, log inclusion | portable picker, current layouts, inventory, plan | evidence and confirmation digests |
| confirm | generation and confirmation digest | exact executable plan, fresh evidence, journal | committed transition receipt |
| recover | request identity | journal, locator, source and target evidence | recovery receipt plus fresh snapshot |
| cleanup | transition id and committed receipt digest | committed receipt, exact paths and hashes | exact removed/already-absent paths |

Conflicts suppress the page confirmation action. Authority remains the final
gate. Locator publication stays last. Cleanup can only be reconstructed from a
committed transition receipt.

## Backup Matrix

| Operation | Required evidence | Host authority | Receipt |
| --- | --- | --- | --- |
| inventory | bounded root scan | archive inspection and same-app check | valid candidates plus preserved diagnostics |
| create | explicit refuse or flush choice | pending publication, capture scope, adapters, encryption, operational target | capture and verified publication |
| export | inventoried archive digest | fresh hash check and injected target picker | user-export publication |
| retention | generation and host-issued digest | product policy, complete listing, exact plan and hash recheck | exact deleted paths |

Only verified operational publication is reported as backup success. Pending
debounce cannot be hidden. Locked, corrupt, foreign, unknown, and unreadable
entries remain outside automatic deletion.

## Composition

`longhorn-settings-config` declares one optional module, one section, two
renderer keys, and independent storage-diagnostics and backup-inventory page
admission. The pages use no ordinary settings scope or apply unit. Mutation
buttons depend on the finer config-operation capabilities in the loaded
snapshot.

`@inflatable-cookie/longhorn-config/poodle` uses public `@poodle/svelte` components. The
framework-neutral root imports no UI or Tauri runtime. `@inflatable-cookie/longhorn-settings`
gains no config dependency.

## Evidence

- Rust-produced protocol fixture covers every command and successful outcome
- fixture inventory covers valid, locked, corrupt, foreign, unknown,
  unreadable, and unmanaged states
- TypeScript validators reject future versions, unknown discriminants, and
  malformed confirmation/archive digests
- serialized client and injected-handler conformance covers all eight commands
- capability examples separate read, storage mutation, and backup mutation
- registration tests cover both, either, and absent base capabilities
- mounted pages cover exact roots, conflict suppression, pending flush,
  preserved inventory, teardown, and SSR import
- source audits exclude Tauri, Poodle, Svelte, age identity, passphrase, and
  archive payload authority from the wrong layers
- package dry run, Rust, TypeScript, Svelte, and Effigy QA

All pass.

## Limits

- host applications still assemble concrete transition execution, idempotency,
  retention policy, pending flush, picker, and encryption providers
- restore inspection and execution remain Card 047
- artifact-installed three-shape composition remains Card 048
