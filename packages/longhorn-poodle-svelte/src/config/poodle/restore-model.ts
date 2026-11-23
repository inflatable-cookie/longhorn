import {
  RESTORE_COMPATIBILITY_LABEL_TEMPLATES,
  renderLabelTemplate,
} from "@inflatable-cookie/longhorn/config";
import type {
  RestoreDomainCompatibilityProjection,
  RestoreDomainInspectionProjection,
  RestoreIdentityStatusProjection,
} from "@inflatable-cookie/longhorn/config";

export type RestoreChoice = "" | "useArchive" | "keepCurrent";

export function canUseArchive(domain: RestoreDomainInspectionProjection): boolean {
  return (
    domain.compatibility.status === "ready" || domain.compatibility.status === "migrationRequired"
  );
}

/**
 * Renders a compatibility classification from the generated template table.
 *
 * The templates come from Rust, where the wording lives on the classification
 * itself, and Rust renders from the same table — so the two backends cannot
 * word a classification differently. Six of the thirteen interpolate their own
 * fields, which is why this is a template table rather than a finished-string
 * map. See memo 022, D2, and Card 170.
 */
export function compatibilityLabel(compatibility: RestoreDomainCompatibilityProjection): string {
  const template = RESTORE_COMPATIBILITY_LABEL_TEMPLATES[compatibility.status];
  return renderLabelTemplate(template, fieldsOf(compatibility));
}

function fieldsOf(compatibility: RestoreDomainCompatibilityProjection): Record<string, string> {
  const fields: Record<string, string> = {};
  for (const [key, value] of Object.entries(compatibility)) {
    if (key !== "status" && (typeof value === "string" || typeof value === "number")) {
      fields[key] = String(value);
    }
  }
  return fields;
}

/**
 * Renders one identity comparison, naming both sides when they differ.
 *
 * Checked against the Rust projection by the shared parity fixture.
 */
export function identityLabel(status: RestoreIdentityStatusProjection): string {
  return status.status === "compatible"
    ? "Compatible"
    : `Mismatch: host expects ${status.expected}, archive declares ${status.archive}`;
}
