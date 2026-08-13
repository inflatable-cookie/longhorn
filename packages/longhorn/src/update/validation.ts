import { UPDATE_FIELDS } from "./generated/fields.ts";
import {
  UPDATE_AVAILABILITY_STATES,
  UPDATE_CHANGED_KINDS,
  UPDATE_CHANNELS,
  UPDATE_DEFERRAL_CAUSES,
  UPDATE_INSTALL_MANAGERS,
  UPDATE_OFFER_REASONS,
  UPDATE_OUTCOME_STATUSES,
  UPDATE_PROGRESS_STATES,
  UPDATE_PROTOCOL_VERSION,
  UPDATE_REJECTION_CODES,
  type UpdateChangedEvent,
  type UpdateCheckCommand,
  type UpdateDeferCommand,
  type UpdateInstallCommand,
  type UpdateOutcomeProjection,
  type UpdateSelectChannelCommand,
  type UpdateSnapshot,
} from "./generated/protocol.ts";
import {
  UPDATE_VARIANT_FIELDS,
  UPDATE_VARIANT_FIELDS_DISCRIMINANTS,
} from "./generated/variant-fields.ts";

export class UpdateValidationError extends Error {
  constructor(
    readonly path: string,
    message: string,
  ) {
    super(`${path}: ${message}`);
    this.name = "UpdateValidationError";
  }
}

export function assertUpdateSnapshot(value: unknown): asserts value is UpdateSnapshot {
  noPayload(value);
  const root = object(value, "$");
  exact(root, "$", UPDATE_FIELDS.UpdateSnapshot);
  protocol(root.protocolVersion, "$.protocolVersion");
  positive(root.authorityEpoch, "$.authorityEpoch");
  oneOf(root.channel, "$.channel", UPDATE_CHANNELS);
  version(root.installedVersion, "$.installedVersion");
  availability(root.availability, "$.availability");
  progress(root.progress, "$.progress");
  if (root.deferral !== null) {
    const deferral = object(root.deferral, "$.deferral");
    exact(deferral, "$.deferral", UPDATE_FIELDS.UpdateDeferralProjection);
    version(deferral.version, "$.deferral.version");
    cause(deferral.cause, "$.deferral.cause");
  }
}

export function assertUpdateOutcome(value: unknown): asserts value is UpdateOutcomeProjection {
  noPayload(value);
  const root = object(value, "$");
  oneOf(root.status, "$.status", UPDATE_OUTCOME_STATUSES);
  exact(root, "$", variantFields("UpdateOutcomeProjection", root, "$"));
  if (root.status === "rejected") oneOf(root.code, "$.code", UPDATE_REJECTION_CODES);
  assertUpdateSnapshot(root.snapshot);
}

export function assertUpdateChangedEvent(value: unknown): asserts value is UpdateChangedEvent {
  noPayload(value);
  const root = object(value, "$");
  exact(root, "$", UPDATE_FIELDS.UpdateChangedEvent);
  protocol(root.protocolVersion, "$.protocolVersion");
  positive(root.authorityEpoch, "$.authorityEpoch");
  oneOf(root.kind, "$.kind", UPDATE_CHANGED_KINDS);
}

export function assertUpdateCheckCommand(value: unknown): asserts value is UpdateCheckCommand {
  commandBase(value, UPDATE_FIELDS.UpdateCheckCommand);
}

export function assertUpdateSelectChannelCommand(
  value: unknown,
): asserts value is UpdateSelectChannelCommand {
  commandBase(value, UPDATE_FIELDS.UpdateSelectChannelCommand);
  oneOf(object(value, "$").channel, "$.channel", UPDATE_CHANNELS);
}

export function assertUpdateDeferCommand(value: unknown): asserts value is UpdateDeferCommand {
  commandBase(value, UPDATE_FIELDS.UpdateDeferCommand);
  const root = object(value, "$");
  version(root.version, "$.version");
  cause(root.cause, "$.cause");
}

export function assertUpdateInstallCommand(value: unknown): asserts value is UpdateInstallCommand {
  commandBase(value, UPDATE_FIELDS.UpdateInstallCommand);
  version(object(value, "$").version, "$.version");
}

function availability(value: unknown, path: string): void {
  const root = object(value, path);
  oneOf(root.state, `${path}.state`, UPDATE_AVAILABILITY_STATES);
  exact(root, path, variantFields("UpdateAvailabilityProjection", root, path));
  if (root.state === "offer") {
    version(root.version, `${path}.version`);
    oneOf(root.reason, `${path}.reason`, UPDATE_OFFER_REASONS);
    optionalString(root.notes, `${path}.notes`);
  }
  // Ahead-of-channel is the one state whose fields a surface must be able to
  // read to say anything useful: "you are on 1.3.0-nightly.4 and production
  // publishes 1.2.9" is the whole message, and Card 154 calls it the single
  // most likely support question the feature generates.
  if (root.state === "aheadOfChannel") {
    version(root.installed, `${path}.installed`);
    version(root.channel, `${path}.channel`);
  }
  if (root.state === "withheldByRollout") version(root.version, `${path}.version`);
  if (root.state === "managedElsewhere") {
    version(root.version, `${path}.version`);
    oneOf(root.manager, `${path}.manager`, UPDATE_INSTALL_MANAGERS);
  }
}

function progress(value: unknown, path: string): void {
  const root = object(value, path);
  oneOf(root.state, `${path}.state`, UPDATE_PROGRESS_STATES);
  exact(root, path, variantFields("UpdateProgressProjection", root, path));
  // Absent rather than zero when the source declares no length. A validator
  // that coerced `null` to 0 here would put back the invented number the
  // protocol went out of its way to avoid.
  if (root.state === "downloading") fraction(root.fraction, `${path}.fraction`);
  if (root.state === "readyToInstall" || root.state === "installing") {
    version(root.version, `${path}.version`);
  }
}

function cause(value: unknown, path: string): void {
  const root = object(value, path);
  oneOf(root.cause, `${path}.cause`, UPDATE_DEFERRAL_CAUSES);
  exact(root, path, variantFields("DeferralCause", root, path));
  if (root.cause === "externallyManaged") {
    oneOf(root.manager, `${path}.manager`, UPDATE_INSTALL_MANAGERS);
    optionalString(root.command, `${path}.command`);
  } else if (root.cause !== "userPostponed") {
    string(root.detail, `${path}.detail`);
  }
}

function commandBase(value: unknown, expected: readonly string[]): void {
  noPayload(value);
  const root = object(value, "$");
  exact(root, "$", expected);
  protocol(root.protocolVersion, "$.protocolVersion");
  positive(root.authorityEpoch, "$.authorityEpoch");
}

/**
 * Allowed keys for one variant, from the generated map.
 *
 * A missing entry means the generator failed to read the union, not that a
 * consumer sent something odd — every caller runs `oneOf` over the
 * discriminant first, so an unknown one is already rejected by then.
 */
function variantFields(
  type: string,
  value: Record<string, unknown>,
  path: string,
): readonly string[] {
  const discriminant = value[UPDATE_VARIANT_FIELDS_DISCRIMINANTS[type] ?? "kind"];
  const fields = UPDATE_VARIANT_FIELDS[type]?.[discriminant as string];
  if (fields === undefined) fail(path, `no generated fields for ${type}.${String(discriminant)}`);
  return fields;
}

function noPayload(value: unknown): void {
  const visit = (candidate: unknown, path: string): void => {
    if (Array.isArray(candidate)) return candidate.forEach((item, index) => visit(item, `${path}[${index}]`));
    if (candidate !== null && typeof candidate === "object")
      for (const [key, child] of Object.entries(candidate)) {
        if (key.toLocaleLowerCase().includes("payload")) fail(`${path}.${key}`, "product payload field is forbidden");
        visit(child, `${path}.${key}`);
      }
  };
  visit(value, "$");
}
function object(value: unknown, path: string): Record<string, unknown> { if (value === null || typeof value !== "object" || Array.isArray(value)) fail(path, "expected object"); return value as Record<string, unknown>; }
function exact(value: Record<string, unknown>, path: string, expected: readonly string[]): void { const actual = Object.keys(value).sort(); const wanted = [...expected].sort(); if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) fail(path, `unexpected keys: ${actual.join(",")}`); }
function protocol(value: unknown, path: string): void { if (value !== UPDATE_PROTOCOL_VERSION) fail(path, `expected exact protocol ${UPDATE_PROTOCOL_VERSION}`); }
function positive(value: unknown, path: string): void { if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) fail(path, "expected positive safe integer"); }
function string(value: unknown, path: string): void { if (typeof value !== "string") fail(path, "expected string"); }
function optionalString(value: unknown, path: string): void { if (value !== null) string(value, path); }
/** Versions are strings on the wire, as `semver::Version` serialises them. */
function version(value: unknown, path: string): void { string(value, path); if ((value as string).length === 0) fail(path, "expected non-empty version"); }
function fraction(value: unknown, path: string): void { if (value === null) return; if (typeof value !== "number" || !Number.isFinite(value) || value < 0 || value > 1) fail(path, "expected a fraction between 0 and 1, or null"); }
function oneOf(value: unknown, path: string, values: readonly string[]): void { if (typeof value !== "string" || !values.includes(value)) fail(path, "unsupported value"); }
function fail(path: string, message: string): never { throw new UpdateValidationError(path, message); }
