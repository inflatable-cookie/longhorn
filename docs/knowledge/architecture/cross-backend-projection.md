# Cross-backend Projection

Status: active
Owner: Tom
Updated: 2026-09-26
Contracts: 005, 013, 016, 020

Longhorn has two permanent, first-class hosts: Tauri (with a Svelte tier) and
GPUI (with Rust projections into `poodle-specs`). Tauri is not legacy; it stays
the fast path for prototypes and for applications that never justify
conversion. Poodle is the UI layer for both hosts (operator rulings,
2026-08-08).

The same Longhorn fact must read the same way on both hosts. These rules settle
the places where the two tiers once diverged (operator rulings 2026-08-09,
plus 2026-08-15 for the sidebar label). The parity fixture checks them on both
sides, and its list of deliberate differences is empty.

## Rules

- **UI text never shows a serde wire form.** Labels come from the owning Rust
  domain enum and are generated into TypeScript, checked by `check:bindings`.
  Where a label interpolates fields, both tiers fill `{name}` placeholders from
  the same generated template. `identityLabel` is the one known exception and
  is still stated twice.
- **Settings search and deep-link resolution live in Rust**
  (`longhorn-poodle::settings`). Page and anchor hits are distinct
  destinations. An unknown anchor is an error, never a silent fall back to the
  top of the page.
- **The host decides case folding, timestamps and request ids** through
  `longhorn_core::HostServices` (`new_request_id`, `format_timestamp`,
  `fold_case`), supplied once at composition. `PlainHostServices` is for tests
  only.
- **Critical stays distinct from Error in text.** A `Critical` record's toast
  title carries `NotificationSeverity::title_prefix` (`"Critical: "`), generated
  into `NOTIFICATION_SEVERITY_TITLE_PREFIXES` and applied by both tiers. A
  severity absent from the map needs no prefix because its tone says enough.
  If a second domain needs a fifth severity level, a Poodle tone becomes the
  right answer.
- **Toasts carry no read state.** Unread-ness belongs to the notification
  centre, where it can be acted on.
- **A toast projects the record's first action**, stated once as `toastAction`
  in `packages/longhorn`. The severity-to-tone rule is stated once as
  `notificationSeverityTone`.
- **A settings sidebar section shows its own label, always.** A host that
  wants its module named writes it into the section label.

## Open

- The Svelte tier re-declares Poodle spec shapes as prop bundles
  (`OperationStatusTone`, `OperationProgressView`). See Q-004 in
  [questions](../questions.md).
