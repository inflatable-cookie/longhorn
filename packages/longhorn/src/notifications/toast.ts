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
