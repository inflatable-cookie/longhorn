# 005 Settings And System Registration

Status: active compiled boundary
Owner: Tom
Updated: 2026-11-24
Evidence: `002-shared-desktop-systems-follow-up.md` (Git history);
`012-settings-registry-and-transaction-boundary.md` (Git history)

## Boundary

Longhorn owns a composable settings registry, authority protocol, session
state, and optional shell. Apps and optional modules own product schemas,
defaults, copy, page bodies, and specialist side effects.

Settings is optional. Its root imports no layout, Surface, command, history,
backend, Svelte, or Poodle package.

## Registry

The host registers modules, sections, pages, and apply units, then seals one
registry generation before serving it.

Each page declares:

- stable namespaced page id and owning module id
- section id, label, keywords, explicit order, and optional stable anchors
- required composed capabilities
- renderer key: a Longhorn built-in or a consumer-owned resolver key
- readable scopes and writable apply units
- reset, import, backup, restore, and confirmation support

Duplicate module, section, page, anchor, or apply-unit ids fail registration.
A page also rejects search keywords that repeat after case folding, and a page
that offers reset fails seal unless one of its writable apply units supports
reset. Order is explicit with stable id as the deterministic tie-break. A
section with no admitted pages disappears.

The sealed registry is immutable for one host generation. Recomposition
creates a new generation. Runtime availability changes page state, not
registry identity.

Capabilities admit registered pages and describe runtime availability. They
do not replace Tauri capability security or host-side caller authorization.
Each load names an admitted page and a scope that page declares readable.
Seal rejects a page that writes an apply unit without reading its scope.
The host checks registry generation and unit/scope identity before page
authorization. Reset additionally requires the page's reset feature and a
reset-capable unit.

## Authority Projection

Rust authority returns checked snapshots. A projection distinguishes:

- configured user value
- effective value
- compiled default
- policy provenance and constraints
- editable, read-only, hidden, or unsupported state
- source diagnostics and recovery state
- authority token and scope revision
- outstanding activation requirements

The renderer never infers effective values or policy precedence. An explicit
managed policy overrides or constrains user configuration. An override blocks
mutation through settings and leaves the stored user value available for
later use. Constraints are enforced when validating intent. Rust and
TypeScript reject wire snapshots with duplicate entry IDs or contradictory
provenance. A managed-policy source requires a policy override and a
non-editable entry. A user-configuration source requires a configured value.

Reset removes the user override in its named scope. It does not mutate
compiled defaults, administrator policy, secrets, or another scope.

## Apply Units

One apply unit is the smallest failure-atomic settings authority.

The built-in config adapter binds one apply unit to exactly one registered
configuration domain. It:

1. accepts a host-issued authority token and typed consumer intent
2. acquires the existing store coordinator
3. rereads and validates current authority
4. rejects a stale token without publication
5. applies the typed patch or reset
6. validates and publishes through `longhorn-config`
7. returns the new authoritative snapshot and exact receipt

Consumer code owns projection, patch, validation, and reset semantics. Opaque
product values may cross the transport under that codec but do not become
Longhorn domain types. Load, apply, and reset request IDs correlate commands
and receipts. Reusing an ID does not deduplicate a command. A load may carry
the last observed authority as an advisory hint and still returns a complete
snapshot on success.

A page with multiple config domains has multiple apply units. Page-level
Apply is allowed only when all dirty state belongs to one unit or an explicit
`ConsumerSettingsTransactionAuthority` promises failure atomicity. Its
`ConsumerSettingsTransactionOutcome` carries the consumer authority receipt.
Sequential multi-domain writes have separate receipts.

## Mutation Timing And Activation

Each apply unit declares one mutation timing mode:

- `immediate`: each accepted intent is sent to authority; pending UI is not a
  persistence receipt
- `staged`: drafts remain renderer-local until explicit Apply; Cancel discards
  them

Restart-required is not a third mutation mode. A successful immediate or
staged receipt may name an activation target such as the app, one window, or a
consumer module. Persistence success and runtime activation remain distinct.
The shell displays outstanding activation until the host reports it satisfied.
Longhorn does not restart a process by inference.

Client validation improves feedback. Host validation remains authoritative.
Invalid staged state cannot apply. A stale apply returns fresh authority and a
conflict; it does not merge or overwrite silently.

## Session And Navigation

One shell instance owns independent route and page-session state. Opening,
closing, or importing a module creates no hidden global singleton.

The shell owns:

- deterministic navigation and search
- structural deep links by page id and optional anchor
- lazy page load and explicit loading, unsupported, reconnecting, recovery,
  conflict, and failure states
- page-switch and close guards for staged dirty state and unsafe in-flight work
- scoped Apply, Cancel, Reset, and confirmation coordination
- focus restoration and accessible status/error announcements

Search indexes registered labels and keywords. It does not inspect consumer
page DOM. Unknown links and missing renderer keys are explicit composition
errors.

Listener registration precedes initial snapshots. Scope revision and registry
generation reject stale updates. Teardown releases listeners and pending
session work exactly once.

## Presentation

The same registry and session controller can be hosted in a modal, independent
window, or routed panel. The shell does not require layout or Surfaces.

Poodle owns dialog, navigation, fields, focus behavior, and visual semantics.
Longhorn uses public controlled props, snippets, events, and presentation
providers. Consumer pages render through registered snippets or resolvers.
Longhorn ships no mandatory app frame and copies no Poodle component.

## Shared Modules

Longhorn may register modules only for implemented capabilities:

- storage-layout diagnostics and profile-transition preview
- backup inventory, creation, export, and retention outcomes
- restore inspection, conflict planning, confirmation, execution, and recovery
- windowing after a generic settings projection exists
- keybindings after g01.010
- backend connection after g01.009

Registration provides stable identities and capability requirements. The
consumer supplies module, section, and page labels and order, plus page
keywords.

Storage and recovery pages preserve contract 004. File selection, secret
identity, destructive confirmation, and host restart remain injected
authority. A shared page cannot bypass confirmation digests, safety backups,
journals, or recovery-required states.

## Errors And Receipts

Registration, load, validation, conflict, policy, capability, mutation,
durability, activation, backup, restore, and recovery failures remain typed.
Config write refusals return settings rejections. Read-only authorities
return `readOnly`. Scopes requiring a separate project authority return
`unauthorized`. Invalid patched values return `invalidIntent`. Unavailable,
corrupt, or migration-blocked scopes return `recoveryRequired`. Operational
failures such as a failed publication remain host errors.
The shell never converts a failed host action into a saved or applied status.

Receipts name the registry generation, page, apply unit, authority token,
mutation outcome, durability where relevant, and activation requirement.
Backup and restore retain their contract-004 receipts rather than a lossy
settings wrapper.

## Package Shape

- `longhorn-settings`: pure registry and authority protocol
- `longhorn-settings-config`: config-domain apply units and shared config
  modules
- `longhorn-tauri-settings`: narrow command/event host assembly
- `@inflatable-cookie/longhorn/settings`: checked protocol, client, projections, and optional
  Svelte/Poodle subpaths
- `@inflatable-cookie/longhorn/config`: checked storage, backup, restore, and recovery client

## Acceptance

- The minimal-shell proof composes one staged preference page without layout or Surfaces
- Soundcheck composes product pages beside shared backup and restore pages
- Loophole composes custom hardware and keybinding pages without moving their
  product authorities into Longhorn
- an app without Surfaces, commands, or a server has no dead navigation
- duplicate ids and missing renderers fail before guarded reveal
- configured, effective, policy, and activation state are not conflated
- stale or invalid staged changes cannot persist
- one-domain apply is failure-atomic and multi-domain limits are visible
- direct links and search resolve stable ids without inspecting page DOM
- modal, window, and panel hosts share one registry/session contract
- mounted teardown leaves no listener or pending session work
- visual implementation uses public Poodle components
