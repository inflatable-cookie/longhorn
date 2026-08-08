import type { NotificationRecordProjection, NotificationSeverityProjection } from "@inflatable-cookie/longhorn/notifications/protocol";

export type NotificationStatusTone = "info" | "success" | "warning" | "danger";

export function notificationStatusTone(severity: NotificationSeverityProjection): NotificationStatusTone {
  return severity === "error" || severity === "critical" ? "danger" : severity;
}

export function notificationStatusLabel(record: NotificationRecordProjection): string {
  return `${record.draft.severity}${record.readState === "unseen" ? ", unseen" : ""}`;
}
