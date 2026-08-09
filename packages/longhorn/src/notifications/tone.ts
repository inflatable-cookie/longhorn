import type { NotificationSeverityProjection } from "./generated/protocol.ts";

/**
 * The tones a notification severity can become.
 *
 * Four, against Longhorn's five severities: `error` and `critical` share
 * `danger` because that is as loud as the tone vocabulary gets.
 */
export type NotificationStatusTone = "info" | "success" | "warning" | "danger";

/**
 * Maps a severity onto a tone.
 *
 * Lives here rather than in a projection tier because two places needed it —
 * `NotificationController.projectToast` and the Poodle projector — and a rule
 * stated twice is a rule that can drift. The cross-backend parity fixture
 * checks this one function, so a third copy would be unchecked. See card 171.
 *
 * The `error`/`critical` collapse is lossy and the tone cannot say so; a
 * surface that needs the distinction restores it in text, as the Rust toast
 * projection does.
 */
export function notificationSeverityTone(
  severity: NotificationSeverityProjection,
): NotificationStatusTone {
  return severity === "error" || severity === "critical" ? "danger" : severity;
}
