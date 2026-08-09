import {
  NOTIFICATION_SEVERITY_LABELS,
  type NotificationRecordProjection,
  type NotificationSeverityProjection,
} from "@inflatable-cookie/longhorn/notifications";

export type NotificationStatusTone = "info" | "success" | "warning" | "danger";

export function notificationStatusTone(severity: NotificationSeverityProjection): NotificationStatusTone {
  return severity === "error" || severity === "critical" ? "danger" : severity;
}

/**
 * The severity's operator-facing name, from the generated map.
 *
 * Previously this returned `record.draft.severity` — the serde wire form, so
 * an operator read `critical` in lowercase. Memo 022, D1.
 */
export function notificationStatusLabel(record: NotificationRecordProjection): string {
  const label = NOTIFICATION_SEVERITY_LABELS[record.draft.severity];
  return `${label}${record.readState === "unseen" ? ", unseen" : ""}`;
}
