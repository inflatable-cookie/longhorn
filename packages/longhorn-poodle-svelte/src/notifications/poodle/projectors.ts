import {
  NOTIFICATION_SEVERITY_LABELS,
  notificationSeverityTone,
  type NotificationRecordProjection,
  type NotificationSeverityProjection,
  type NotificationStatusTone,
} from "@inflatable-cookie/longhorn/notifications";

/**
 * Re-exported from the client tier, where the rule lives so that
 * `NotificationController.projectToast` and this projector cannot disagree.
 */
export function notificationStatusTone(
  severity: NotificationSeverityProjection,
): NotificationStatusTone {
  return notificationSeverityTone(severity);
}

/**
 * The severity's operator-facing name, from the generated map.
 *
 * Previously this returned `record.draft.severity` — the serde wire form, so
 * an operator read `critical` in lowercase (memo 022, D1) — and appended
 * ", unseen" for an unread record. Read state now belongs to the notification
 * centre alone: a toast appears *because* something just happened, so marking
 * it unseen says nothing. Memo 022, D7.
 */
export function notificationStatusLabel(record: NotificationRecordProjection): string {
  return NOTIFICATION_SEVERITY_LABELS[record.draft.severity];
}
