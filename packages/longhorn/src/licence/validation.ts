import { LICENCE_FIELDS } from "./generated/fields.ts";
import {
  LICENCE_CHANGED_KINDS,
  LICENCE_CREDENTIAL_KINDS,
  LICENCE_OUTCOME_STATUSES,
  LICENCE_PROTOCOL_VERSION,
  LICENCE_REJECTION_CODES,
  LICENCE_TRUST_BASIS_KINDS,
  LICENCE_USABILITY_STATES,
  type LicenceActivateCommand,
  type LicenceChangedEvent,
  type LicenceDeactivateCommand,
  type LicenceOutcomeProjection,
  type LicenceRefreshCommand,
  type LicenceSnapshot,
} from "./generated/protocol.ts";
import {
  LICENCE_VARIANT_FIELDS,
  LICENCE_VARIANT_FIELDS_DISCRIMINANTS,
} from "./generated/variant-fields.ts";

export class LicenceValidationError extends Error {
  constructor(
    readonly path: string,
    message: string,
  ) {
    super(`${path}: ${message}`);
    this.name = "LicenceValidationError";
  }
}

export function assertLicenceSnapshot(value: unknown): asserts value is LicenceSnapshot {
  noCredential(value);
  const root = object(value, "$");
  exact(root, "$", LICENCE_FIELDS.LicenceSnapshot);
  protocol(root.protocolVersion, "$.protocolVersion");
  positive(root.authorityEpoch, "$.authorityEpoch");
  // Absent means unlicensed, which is the absence of a licence rather than a
  // licence that cannot be used. Card 193 kept those apart deliberately.
  if (root.licence !== null) held(root.licence, "$.licence");
}

export function assertLicenceOutcome(value: unknown): asserts value is LicenceOutcomeProjection {
  noCredential(value);
  const root = object(value, "$");
  oneOf(root.status, "$.status", LICENCE_OUTCOME_STATUSES);
  exact(root, "$", variantFields("LicenceOutcomeProjection", root, "$"));
  if (root.status === "rejected") oneOf(root.code, "$.code", LICENCE_REJECTION_CODES);
  assertLicenceSnapshot(root.snapshot);
}

export function assertLicenceChangedEvent(value: unknown): asserts value is LicenceChangedEvent {
  noCredential(value);
  const root = object(value, "$");
  exact(root, "$", LICENCE_FIELDS.LicenceChangedEvent);
  protocol(root.protocolVersion, "$.protocolVersion");
  positive(root.authorityEpoch, "$.authorityEpoch");
  oneOf(root.kind, "$.kind", LICENCE_CHANGED_KINDS);
}

export function assertLicenceActivateCommand(
  value: unknown,
): asserts value is LicenceActivateCommand {
  const root = commandBase(value, LICENCE_FIELDS.LicenceActivateCommand);
  const credential = object(root.credential, "$.credential");
  oneOf(credential.kind, "$.credential.kind", LICENCE_CREDENTIAL_KINDS);
  exact(
    credential,
    "$.credential",
    variantFields("LicenceCredentialProjection", credential, "$.credential"),
  );
  // The one place credential material crosses, and it crosses inward only.
  // Checked for shape, never echoed: `noCredential` runs on everything coming
  // back and would reject a projection that returned any of this.
  if (credential.kind === "key") nonEmpty(credential.key, "$.credential.key");
  if (credential.kind === "accountToken") nonEmpty(credential.token, "$.credential.token");
  if (credential.kind === "licenceFile") {
    nonEmpty(credential.contentsBase64, "$.credential.contentsBase64");
  }
}

export function assertLicenceDeactivateCommand(
  value: unknown,
): asserts value is LicenceDeactivateCommand {
  commandBase(value, LICENCE_FIELDS.LicenceDeactivateCommand);
}

export function assertLicenceRefreshCommand(
  value: unknown,
): asserts value is LicenceRefreshCommand {
  commandBase(value, LICENCE_FIELDS.LicenceRefreshCommand);
}

function held(value: unknown, path: string): void {
  const licence = object(value, path);
  exact(licence, path, LICENCE_FIELDS.HeldLicenceProjection);
  string(licence.product, `${path}.product`);
  usability(licence.usability, `${path}.usability`);
  trustBasis(licence.trustBasis, `${path}.trustBasis`);
  // Both windows, kept apart. A perpetual licence past its update window still
  // runs, and one field cannot say that.
  optionalTimestamp(licence.useUntil, `${path}.useUntil`);
  optionalTimestamp(licence.updateUntil, `${path}.updateUntil`);
  array(licence.entitlements, `${path}.entitlements`).forEach((entry, index) => {
    const at = `${path}.entitlements[${index}]`;
    const entitlement = object(entry, at);
    exact(entitlement, at, LICENCE_FIELDS.LicenceEntitlementProjection);
    // Opaque by design: Longhorn enumerates no features, so this checks that
    // an id is present and never what it says.
    nonEmpty(entitlement.id, `${at}.id`);
    if (entitlement.atMost !== null) positive(entitlement.atMost, `${at}.atMost`);
  });
}

function usability(value: unknown, path: string): void {
  const root = object(value, path);
  oneOf(root.state, `${path}.state`, LICENCE_USABILITY_STATES);
  exact(root, path, variantFields("LicenceUsabilityProjection", root, path));
  // `clockRefused` carries no timestamp on purpose: a licence refused because
  // the machine clock moved is not expired, and giving it a date would invite
  // a surface to render it as one.
  if (root.state === "inGrace") timestamp(root.until, `${path}.until`);
  if (root.state === "useWindowExpired" || root.state === "leaseLapsed") {
    timestamp(root.at, `${path}.at`);
  }
}

function trustBasis(value: unknown, path: string): void {
  const root = object(value, path);
  oneOf(root.kind, `${path}.kind`, LICENCE_TRUST_BASIS_KINDS);
  exact(root, path, variantFields("LicenceTrustBasisProjection", root, path));
  if (root.kind === "remoteAssertion") timestamp(root.checked, `${path}.checked`);
}

function commandBase(value: unknown, expected: readonly string[]): Record<string, unknown> {
  const root = object(value, "$");
  exact(root, "$", expected);
  protocol(root.protocolVersion, "$.protocolVersion");
  positive(root.authorityEpoch, "$.authorityEpoch");
  return root;
}

/**
 * Refuses anything that looks like credential material.
 *
 * The rule the licence protocol is built on: credentials travel inward on an
 * activate command and never come back. This runs on every inbound projection,
 * so a future field that leaked a token or a signature fails the boundary
 * rather than reaching a surface that might display it.
 *
 * Deliberately name-based and deliberately broad. A false positive here is a
 * field that has to be renamed; a false negative is a secret on a screen.
 */
function noCredential(value: unknown): void {
  const forbidden = ["credential", "secret", "signature", "token", "privatekey", "keyid"];
  const visit = (candidate: unknown, path: string): void => {
    if (Array.isArray(candidate)) return candidate.forEach((item, index) => visit(item, `${path}[${index}]`));
    if (candidate !== null && typeof candidate === "object")
      for (const [key, child] of Object.entries(candidate)) {
        const lowered = key.toLocaleLowerCase();
        if (forbidden.some((needle) => lowered.includes(needle))) {
          fail(`${path}.${key}`, "credential material must not appear in a projection");
        }
        visit(child, `${path}.${key}`);
      }
  };
  visit(value, "$");
}

function variantFields(
  type: string,
  value: Record<string, unknown>,
  path: string,
): readonly string[] {
  const discriminant = value[LICENCE_VARIANT_FIELDS_DISCRIMINANTS[type] ?? "kind"];
  const fields = LICENCE_VARIANT_FIELDS[type]?.[discriminant as string];
  if (fields === undefined) fail(path, `no generated fields for ${type}.${String(discriminant)}`);
  return fields;
}

function object(value: unknown, path: string): Record<string, unknown> { if (value === null || typeof value !== "object" || Array.isArray(value)) fail(path, "expected object"); return value as Record<string, unknown>; }
function array(value: unknown, path: string): unknown[] { if (!Array.isArray(value)) fail(path, "expected array"); return value; }
function exact(value: Record<string, unknown>, path: string, expected: readonly string[]): void { const actual = Object.keys(value).sort(); const wanted = [...expected].sort(); if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) fail(path, `unexpected keys: ${actual.join(",")}`); }
function protocol(value: unknown, path: string): void { if (value !== LICENCE_PROTOCOL_VERSION) fail(path, `expected exact protocol ${LICENCE_PROTOCOL_VERSION}`); }
function positive(value: unknown, path: string): void { if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) fail(path, "expected positive safe integer"); }
function string(value: unknown, path: string): void { if (typeof value !== "string") fail(path, "expected string"); }
function nonEmpty(value: unknown, path: string): void { string(value, path); if ((value as string).length === 0) fail(path, "expected a non-empty string"); }
/** Unix seconds, and negative is a real point in time. */
function timestamp(value: unknown, path: string): void { if (typeof value !== "number" || !Number.isSafeInteger(value)) fail(path, "expected a unix-second timestamp"); }
function optionalTimestamp(value: unknown, path: string): void { if (value !== null) timestamp(value, path); }
function oneOf(value: unknown, path: string, values: readonly string[]): void { if (typeof value !== "string" || !values.includes(value)) fail(path, "unsupported value"); }
function fail(path: string, message: string): never { throw new LicenceValidationError(path, message); }
