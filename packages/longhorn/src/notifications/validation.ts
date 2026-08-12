import {
  NOTIFICATION_VARIANT_FIELDS,
  NOTIFICATION_VARIANT_FIELDS_DISCRIMINANTS,
} from "./generated/variant-fields.ts";
import { NOTIFICATIONS_FIELDS } from "./generated/fields.ts";
import {
  NOTIFICATION_CHANGED_KINDS,
  NOTIFICATION_MUTATION_KINDS,
  NOTIFICATION_MUTATION_STATUSES,
  NOTIFICATION_PROTOCOL_VERSION,
  NOTIFICATION_READ_STATES,
  NOTIFICATION_REJECTION_CODES,
  NOTIFICATION_RETENTION_CLASSES,
  NOTIFICATION_SEVERITIES,
  type NotificationChangedEvent,
  type NotificationMutationCommand,
  type NotificationMutationResult,
  type NotificationRecordProjection,
  type NotificationSnapshot,
  type NotificationSnapshotQuery,
  type NotificationSnapshotResponse,
} from "./generated/protocol.ts";

export class NotificationProtocolValidationError extends Error {
  constructor(readonly path: string, message: string) {
    super(`incompatible notification protocol at ${path}: ${message}`);
    this.name = "NotificationProtocolValidationError";
  }
}

export function assertValidNotificationSnapshotQuery(value: unknown): asserts value is NotificationSnapshotQuery {
  const object = exact(value, "$", NOTIFICATIONS_FIELDS.NotificationSnapshotQuery);
  protocol(object.protocolVersion, "$.protocolVersion");
  text(object.requestId, "$.requestId");
  natural(object.offset, "$.offset");
  natural(object.limit, "$.limit");
}

export function assertValidNotificationSnapshotResponse(value: unknown): asserts value is NotificationSnapshotResponse {
  const object = exact(value, "$", NOTIFICATIONS_FIELDS.NotificationSnapshotResponse);
  text(object.requestId, "$.requestId");
  assertValidNotificationSnapshot(object.snapshot);
}

export function assertValidNotificationSnapshot(value: unknown): asserts value is NotificationSnapshot {
  const object = exact(value, "$", NOTIFICATIONS_FIELDS.NotificationSnapshot);
  protocol(object.protocolVersion, "$.protocolVersion");
  authority(object.authority, "$.authority");
  natural(object.ledgerRevision, "$.ledgerRevision");
  limits(object.limits, "$.limits");
  natural(object.retainedCount, "$.retainedCount");
  natural(object.unseenCount, "$.unseenCount");
  natural(object.retainedEncodedWeight, "$.retainedEncodedWeight");
  natural(object.prunedCount, "$.prunedCount");
  const page = exact(object.page, "$.page", NOTIFICATIONS_FIELDS.NotificationPageProjection);
  natural(page.offset, "$.page.offset");
  natural(page.totalCount, "$.page.totalCount");
  boolean(page.hasMore, "$.page.hasMore");
  array(page.records, "$.page.records").forEach((item, index) => recordProjection(item, `$.page.records[${index}]`));
}

export function assertValidNotificationMutationCommand(value: unknown): asserts value is NotificationMutationCommand {
  const object = record(value, "$");
  member(object.kind, NOTIFICATION_MUTATION_KINDS, "$.kind");
  protocol(object.protocolVersion, "$.protocolVersion");
  text(object.requestId, "$.requestId");
  authority(object.authority, "$.authority");
  natural(object.expectedLedgerRevision, "$.expectedLedgerRevision");
  switch (object.kind) {
    case "add":
      exactKeys(object, "$", variantKeys("NotificationMutationCommand", object, "$"));
      text(object.notificationId, "$.notificationId");
      draft(object.draft, "$.draft");
      break;
    case "replace":
      exactKeys(object, "$", variantKeys("NotificationMutationCommand", object, "$"));
      draft(object.draft, "$.draft");
      boolean(object.markUnseen, "$.markUnseen");
      break;
    case "markSeen":
    case "dismiss":
      exactKeys(object, "$", variantKeys("NotificationMutationCommand", object, "$"));
      text(object.notificationId, "$.notificationId");
      break;
    case "clear": {
      exactKeys(object, "$", variantKeys("NotificationMutationCommand", object, "$"));
      const target = record(object.target, "$.target");
      member(target.kind, ["all", "records"] as const, "$.target.kind");
      exactKeys(target, "$.target", variantKeys("NotificationClearTargetProjection", target, "$.target"));
      if (target.kind === "records") array(target.notificationIds, "$.target.notificationIds").forEach((id, index) => text(id, `$.target.notificationIds[${index}]`));
      break;
    }
    case "changeRetention":
      exactKeys(object, "$", variantKeys("NotificationMutationCommand", object, "$"));
      limits(object.limits, "$.limits");
      break;
  }
}

export function assertValidNotificationMutationResult(value: unknown): asserts value is NotificationMutationResult {
  const object = record(value, "$");
  member(object.status, NOTIFICATION_MUTATION_STATUSES, "$.status");
  text(object.requestId, "$.requestId");
  assertValidNotificationSnapshot(object.snapshot);
  if (object.status === "committed") {
    exactKeys(object, "$", variantKeys("NotificationMutationResult", object, "$"));
    receipt(object.receipt, "$.receipt");
  } else {
    exactKeys(object, "$", variantKeys("NotificationMutationResult", object, "$"));
    const rejection = exact(object.rejection, "$.rejection", NOTIFICATIONS_FIELDS.NotificationRejection);
    member(rejection.code, NOTIFICATION_REJECTION_CODES, "$.rejection.code");
    text(rejection.detail, "$.rejection.detail");
    boolean(rejection.refreshRequired, "$.rejection.refreshRequired");
  }
}

export function assertValidNotificationChangedEvent(value: unknown): asserts value is NotificationChangedEvent {
  const object = exact(value, "$", NOTIFICATIONS_FIELDS.NotificationChangedEvent);
  protocol(object.protocolVersion, "$.protocolVersion");
  text(object.requestId, "$.requestId");
  authority(object.authority, "$.authority");
  natural(object.previousLedgerRevision, "$.previousLedgerRevision");
  natural(object.committedLedgerRevision, "$.committedLedgerRevision");
  array(object.affectedNotificationIds, "$.affectedNotificationIds").forEach((id, index) => text(id, `$.affectedNotificationIds[${index}]`));
  member(object.kind, NOTIFICATION_CHANGED_KINDS, "$.kind");
}

function authority(value: unknown, path: string): void {
  const object = exact(value, path, NOTIFICATIONS_FIELDS.NotificationAuthorityProjection);
  text(object.authorityId, `${path}.authorityId`);
  positive(object.authorityEpoch, `${path}.authorityEpoch`);
}

function limits(value: unknown, path: string): void {
  const object = exact(value, path, NOTIFICATIONS_FIELDS.NotificationLedgerLimitsProjection);
  natural(object.maximumNotifications, `${path}.maximumNotifications`);
  natural(object.maximumEncodedWeight, `${path}.maximumEncodedWeight`);
}

function draft(value: unknown, path: string): void {
  const object = exact(value, path, NOTIFICATIONS_FIELDS.NotificationDraftProjection);
  text(object.sourceId, `${path}.sourceId`);
  member(object.severity, NOTIFICATION_SEVERITIES, `${path}.severity`);
  text(object.title, `${path}.title`);
  text(object.summary, `${path}.summary`);
  nullableText(object.causeId, `${path}.causeId`);
  array(object.actions, `${path}.actions`).forEach((action, index) => {
    const item = exact(action, `${path}.actions[${index}]`, NOTIFICATIONS_FIELDS.NotificationActionProjection);
    text(item.referenceId, `${path}.actions[${index}].referenceId`);
    text(item.label, `${path}.actions[${index}].label`);
  });
  nullableText(object.replacementKey, `${path}.replacementKey`);
  nullableText(object.producerToken, `${path}.producerToken`);
  member(object.retentionClass, NOTIFICATION_RETENTION_CLASSES, `${path}.retentionClass`);
  if (object.presentationTimeUnixMs !== null) integer(object.presentationTimeUnixMs, `${path}.presentationTimeUnixMs`);
}

function recordProjection(value: unknown, path: string): asserts value is NotificationRecordProjection {
  const object = exact(value, path, NOTIFICATIONS_FIELDS.NotificationRecordProjection);
  text(object.notificationId, `${path}.notificationId`);
  draft(object.draft, `${path}.draft`);
  positive(object.sequence, `${path}.sequence`);
  positive(object.lastChangedLedgerRevision, `${path}.lastChangedLedgerRevision`);
  member(object.readState, NOTIFICATION_READ_STATES, `${path}.readState`);
  natural(object.encodedMetadataWeight, `${path}.encodedMetadataWeight`);
}

function receipt(value: unknown, path: string): void {
  const object = record(value, path);
  member(object.kind, ["added", "replaced", "seen", "removed", "retentionChanged"] as const, `${path}.kind`);
  natural(object.previousLedgerRevision, `${path}.previousLedgerRevision`);
  natural(object.committedLedgerRevision, `${path}.committedLedgerRevision`);
  if (object.kind === "added" || object.kind === "replaced") {
    exactKeys(object, path, variantKeys("NotificationMutationReceiptProjection", object, path));
    recordProjection(object.record, `${path}.record`);
    removals(object.pruned, `${path}.pruned`);
  } else if (object.kind === "seen") {
    exactKeys(object, path, variantKeys("NotificationMutationReceiptProjection", object, path));
    recordProjection(object.record, `${path}.record`);
  } else if (object.kind === "removed") {
    exactKeys(object, path, variantKeys("NotificationMutationReceiptProjection", object, path));
    removals(object.removals, `${path}.removals`);
  } else {
    exactKeys(object, path, variantKeys("NotificationMutationReceiptProjection", object, path));
    limits(object.previousLimits, `${path}.previousLimits`);
    limits(object.committedLimits, `${path}.committedLimits`);
    removals(object.removals, `${path}.removals`);
  }
}

function removals(value: unknown, path: string): void {
  array(value, path).forEach((removal, index) => {
    const item = exact(removal, `${path}[${index}]`, NOTIFICATIONS_FIELDS.NotificationRemovalProjection);
    recordProjection(item.record, `${path}[${index}].record`);
    member(item.reason, ["dismissed", "cleared", "pruned"] as const, `${path}[${index}].reason`);
  });
}

function protocol(value: unknown, path: string): void {
  if (value !== NOTIFICATION_PROTOCOL_VERSION) fail(path, `expected protocol ${NOTIFICATION_PROTOCOL_VERSION}`);
}

function record(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) fail(path, "expected object");
  return value as Record<string, unknown>;
}

function exact(value: unknown, path: string, keys: readonly string[]): Record<string, unknown> {
  const object = record(value, path);
  exactKeys(object, path, keys);
  return object;
}

function exactKeys(object: Record<string, unknown>, path: string, keys: readonly string[]): void {
  const actual = Object.keys(object).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) fail(path, `expected keys ${expected.join(", ")}`);
}

function array(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) fail(path, "expected array");
  return value;
}

function text(value: unknown, path: string): void { if (typeof value !== "string" || value.length === 0) fail(path, "expected nonempty string"); }
function nullableText(value: unknown, path: string): void { if (value !== null) text(value, path); }
function boolean(value: unknown, path: string): void { if (typeof value !== "boolean") fail(path, "expected boolean"); }
function integer(value: unknown, path: string): void { if (typeof value !== "number" || !Number.isSafeInteger(value)) fail(path, "expected safe integer"); }
function natural(value: unknown, path: string): void { integer(value, path); if ((value as number) < 0) fail(path, "expected nonnegative integer"); }
function positive(value: unknown, path: string): void { natural(value, path); if ((value as number) === 0) fail(path, "expected positive integer"); }
function member(value: unknown, members: readonly string[], path: string): void { if (typeof value !== "string" || !members.includes(value)) fail(path, `expected one of ${members.join(", ")}`); }
function fail(path: string, message: string): never { throw new NotificationProtocolValidationError(path, message); }

/**
 * Allowed keys for one tagged-union variant, from the generated map, with the
 * discriminant's name read from the map too.
 *
 * A missing entry means the generator failed to read the union: every caller
 * checks the discriminant above this call.
 */
function variantKeys(type: string, object: Record<string, unknown>, path: string): readonly string[] {
  const discriminant = object[NOTIFICATION_VARIANT_FIELDS_DISCRIMINANTS[type] ?? "kind"];
  const keys = NOTIFICATION_VARIANT_FIELDS[type]?.[discriminant as string];
  if (keys === undefined) fail(path, `no generated fields for ${type}.${String(discriminant)}`);
  return keys;
}
