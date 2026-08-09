import { NOTIFICATION_SEVERITY_TITLE_PREFIXES } from "./generated/labels.ts";

import type {
  NotificationActionProjection,
  NotificationDraftProjection,
} from "./generated/protocol.ts";

/**
 * The one action a toast can carry, if the record has any.
 *
 * A `NotificationRecord` may declare several; a toast has room for one. The
 * first is taken and the rest stay reachable from the notification centre —
 * a presentation choice, not a loss of record.
 *
 * Stated here rather than inlined so both this tier's controller and the Rust
 * projection answer to the same rule, and so the cross-backend parity fixture
 * can check it. Memo 022, D9.
 */
export function toastAction(
  draft: NotificationDraftProjection,
): NotificationActionProjection | undefined {
  return draft.actions[0];
}

/**
 * The title a toast shows, with the severity said in words where the tone
 * cannot say it.
 *
 * `Critical` and `Error` share the `danger` tone — four tones against five
 * severities — so on screen they are the same tint and the same weight. A
 * surface that renders the tone alone tells an operator that a read-only
 * volume is the same class of problem as a failed sync.
 *
 * The prefixes are generated from `NotificationSeverity::title_prefix`, so
 * both backends mark the same titles. A severity absent from the map needs no
 * prefix: its tone says enough. Memo 022, D5.
 */
export function toastTitle(draft: NotificationDraftProjection): string {
  const prefix = NOTIFICATION_SEVERITY_TITLE_PREFIXES[draft.severity] ?? "";
  return `${prefix}${draft.title}`;
}
