# 005 Settings And System Registration

Status: active first pass  
Owner: Tom  
Updated: 2026-07-27

## Contract

Longhorn provides one settings shell. Apps and optional Longhorn modules
register sections and pages.

Each page declares:

- stable id, label, keywords, order, and owning module
- required capabilities
- configuration domains and scopes it reads or mutates
- validation and dirty-state policy
- immediate, staged, or restart-required application behavior
- reset and restore support

The shell owns navigation, search, deep links, dirty-state protection,
apply/cancel flow, errors, and accessibility. Poodle owns dialog, field,
navigation, and presentation primitives.

## Authority

- Rust configuration domains remain authoritative.
- Page renderers may stage drafts but never persist files directly.
- Apps own product-specific pages and copy.
- Longhorn modules may register generic pages such as windowing, keybindings,
  storage/backup, or backend connection.
- Registration failure is explicit. Duplicate ids never resolve by order.

## Composition

- A minimal app may register one page.
- Optional modules disappear cleanly when absent.
- Settings can open as a modal, window, or routed panel over the same registry.
- The shell does not require Surface or layout packages.
- Backup/restore and reset are scoped operations with confirmation and result
  receipts.

## Acceptance

- Loophole keybindings and a Bovine preference page coexist through the same
  registry shape
- an app without Surfaces or a server has no empty pages for them
- staged invalid changes cannot close or persist silently
- direct-link and search results resolve stable page ids
- visual implementation uses Poodle public components

