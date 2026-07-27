# g01.008 Settings Registry And Shell

Status: blocked on `g01.002` and `g01.007`  
Owner: Tom  
Updated: 2026-07-27  
Governing refs: contract 005

## Outcome

Provide one composable settings experience for app and optional-module
configuration.

## Batches

### 1. Registry and transactions

- stable sections/pages, ordering, keywords, capabilities, deep links
- immediate, staged, and restart-required policies
- validation, dirty state, reset, and scoped apply/cancel

### 2. Shell and Poodle adapters

- navigation, search, errors, and accessibility
- modal, window, and panel hosts over one registry
- extension slots for app-rendered pages

### 3. Shared pages

- storage/backup/restore and diagnostics
- windowing where composed
- keybindings after command system lands
- backend connection where composed

### 4. Consumer fixtures

- Loophole keybinding/settings shape
- Soundcheck product settings
- Bovine minimal preference page

## Acceptance

- absent modules produce no dead navigation
- invalid staged changes do not persist
- all persistence passes through configuration commands
- product pages retain app ownership and use Poodle primitives

