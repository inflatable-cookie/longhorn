# Shared System Suite And g01 Runway

Date: 2026-07-27  
State: complete planning batch

## Outcome

- preserved configuration, backup, settings, optional server, command
  palette, and history requirements
- inspected Loophole's live history and configuration donors
- corrected forkable history from assumed donor behavior to gated research
- promoted configuration, settings, command/input, topology, and history
  boundaries into contracts 004-008
- expanded architecture, inventory, authority, validation, and planning gaps
- compiled 16 g01 milestones from foundation contracts through consumer
  migrations and first release

## Evidence

- `../../research/translation-memos/002-shared-desktop-systems-follow-up.md`
- `../../specs/001-shared-desktop-system-suite.md`
- Loophole `pulse-history`, `echo-configuration`, action/input, settings, and
  command runtime sources
- Soundcheck settings/window persistence
- Bovine Tauri config-root preference storage

## Decisions

- configuration is a registered-domain system, not one shared settings file
- secrets, caches, local UI state, and server-owned data have distinct classes
- backup is registry-driven and verified; restore is staged and transactional
- settings pages register into one shell while product pages remain app-owned
- command palette, menus, shortcuts, and settings project one command registry
- server topology is optional and cannot capture local desktop authority
- linear history may be extracted; branching needs a separate prototype

## Next

Review contract 004, close the remaining `g01.001` contracts, and freeze the
package graph.
