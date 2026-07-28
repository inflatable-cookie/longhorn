# 011 Platform Storage Layout Profiles

Status: complete
Owner: Tom
Roadmap: g01.002 batch 5
Governing refs: contracts 001, 004, and 012; research memo 007
Auto-start next card: no

## Objective

Resolve complete, versioned storage layouts for macOS, Windows, and Linux from
immutable app identity and one selected policy profile.

## Scope

- immutable application id and optional stable storage name
- canonical-default leaf selection with explicit provenance
- config, data, state, cache, log, runtime, backup, policy, workspace, and
  project root purposes
- pure injected platform-directory facts
- `platform-native-v1`, `unified-app-root-v1`, and `portable-v1`
- explicit typed per-purpose overrides with provenance
- layout digest, warnings, failures, and diagnostic projection
- `StorageRoots` integration, including a distinct state root
- durable database-directory convention without database I/O
- narrow Tauri platform-facts adapter
- macOS, Windows, Linux, and donor-identity path matrices

## Public Behavior

`platform-native-v1` is the default and immutable. Every app-specific leaf
defaults to canonical app id. One optional stable storage name replaces that
leaf throughout the resolved layout. Windows uses LocalAppData with no roaming
claim. Linux uses XDG config, data, state, cache, and runtime bases.

Unified and portable layouts produce all typed children without individual
path overrides. Portable resolution requires one explicit absolute root.

Resolution is pure. It creates no directories and reads no process
environment. The Tauri adapter supplies platform facts at the host edge.

## Out Of Scope

- persistent profile locator and settings UI
- moving, importing, or deleting existing files
- database snapshot or restore
- consumer repository writes
- remote or roaming synchronization
- custom user-authored profile language

## Acceptance Criteria

- exact path matrices cover all built-in profiles on all three platforms
- relative, empty, or missing required facts fail with typed diagnostics
- profile ids and layout digests are deterministic
- display-name changes cannot move storage
- canonical id supplies every leaf when no storage name is registered
- one stable storage name replaces every leaf without per-purpose overrides
- leaf provenance distinguishes canonical default from explicit storage name
- invalid explicit storage name fails without canonical fallback
- Windows uses the canonical or stable leaf directly and never defaults to
  Roaming
- unified and portable layouts require no per-purpose overrides
- every override and warning appears in the resolved receipt
- machine state no longer aliases durable data by contract
- durable, state, and rebuildable database examples map to data, state, and
  cache
- Tauri remains outside pure resolution
- existing config behavior passes after explicit root injection

## Stop Conditions

- a profile needs ambient environment or filesystem access
- a path derives from the live display name, executable, or current directory
- an existing profile id changes meaning
- database lifecycle must be inferred from file extension
- implementation expands into profile migration or consumer cutover

## Next Task

Card 009 is ready again. Do not auto-start it.
