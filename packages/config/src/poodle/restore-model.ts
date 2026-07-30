import type {
  RestoreDomainCompatibilityProjection,
  RestoreDomainInspectionProjection,
} from "../index.ts";

export type RestoreChoice = "" | "useArchive" | "keepCurrent";

export function canUseArchive(
  domain: RestoreDomainInspectionProjection,
): boolean {
  return (
    domain.compatibility.status === "ready" ||
    domain.compatibility.status === "migrationRequired"
  );
}

export function compatibilityLabel(
  compatibility: RestoreDomainCompatibilityProjection,
): string {
  switch (compatibility.status) {
    case "ready":
      return "Ready";
    case "migrationRequired":
      return `Migration required (${compatibility.from} → ${compatibility.to})`;
    case "unknownDomain":
      return "Unknown domain";
    case "descriptorMismatch":
      return "Descriptor mismatch";
    case "domainCodeUnavailable":
      return "Domain code unavailable";
    case "policyExcluded":
      return `Policy excluded: ${compatibility.reason}`;
    case "customAdapterUnavailable":
      return `Adapter unavailable: ${compatibility.adapter}`;
    case "customAdapterReady":
      return `Custom adapter ready: ${compatibility.adapter}`;
    case "customAdapterRejected":
      return `Adapter rejected: ${compatibility.detail}`;
    case "targetUnavailable":
      return `Target unavailable: ${compatibility.reason}`;
    case "sourcePreserved":
      return `Source preserved: ${compatibility.issue}`;
    case "sourceRejected":
      return `Source rejected: ${compatibility.issue}`;
    case "targetPreparationFailed":
      return `Target preparation failed: ${compatibility.detail}`;
  }
}

