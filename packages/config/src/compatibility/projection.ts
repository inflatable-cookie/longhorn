import {
  BACKUP_ENCRYPTION_STATES,
  BACKUP_PENDING_STATES,
  CONFIG_OPERATION_CAPABILITIES,
  RESTORE_ADAPTER_PARTICIPATION_KINDS,
  RESTORE_CURRENT_EVIDENCE_STATES,
  RESTORE_DOMAIN_COMPATIBILITY_STATUSES,
  RESTORE_IDENTITY_STATUSES,
  STORAGE_BOOTSTRAP_STATES,
  type ConfigOperationsSnapshot,
} from "../generated/protocol.ts";
import {
  array,
  boolean,
  digest,
  discriminant,
  finiteNumber,
  nonempty,
  nullableDigest,
  nullableString,
  protocol,
  record,
  string,
} from "./primitives.ts";

const STORAGE_PROFILES = [
  "platform-native-v1",
  "unified-app-root-v1",
  "portable-v1",
] as const;

export function assertCompatibleConfigOperationsSnapshot(
  value: unknown,
): asserts value is ConfigOperationsSnapshot {
  const snapshot = record(value, "$");
  protocol(snapshot.protocolVersion, "$.protocolVersion");
  finiteNumber(snapshot.generation, "$.generation");
  array(snapshot.capabilities, "$.capabilities").forEach((capability, index) =>
    discriminant(
      capability,
      CONFIG_OPERATION_CAPABILITIES,
      `$.capabilities[${index}]`,
    ),
  );
  if (snapshot.storage !== null) storage(snapshot.storage, "$.storage");
  if (snapshot.backup !== null) backup(snapshot.backup, "$.backup");
  if (snapshot.restore !== null) restoreState(snapshot.restore, "$.restore");
}

export function assertRestoreInspection(value: unknown, path: string): void {
  const inspection = record(value, path);
  digest(inspection.archiveSha256, `${path}.archiveSha256`);
  [
    "archiveId",
    "createdAt",
    "kind",
    "applicationVersion",
    "producerVersion",
  ].forEach((key) => nonempty(inspection[key], `${path}.${key}`));
  discriminant(inspection.integrity, ["verified"], `${path}.integrity`);
  discriminant(
    inspection.authenticity,
    ["unauthenticated", "authenticated"],
    `${path}.authenticity`,
  );
  identity(record(inspection.identity, `${path}.identity`), `${path}.identity`);
  array(inspection.consistencyGroups, `${path}.consistencyGroups`).forEach(
    (value, index) => {
      const groupPath = `${path}.consistencyGroups[${index}]`;
      const group = record(value, groupPath);
      ["id", "mode", "authority"].forEach((key) =>
        nonempty(group[key], `${groupPath}.${key}`),
      );
    },
  );
  array(inspection.domains, `${path}.domains`).forEach((value, index) =>
    restoreDomain(value, `${path}.domains[${index}]`),
  );
  array(inspection.exclusions, `${path}.exclusions`).forEach((value, index) => {
    const exclusionPath = `${path}.exclusions[${index}]`;
    const exclusion = record(value, exclusionPath);
    ["domainId", "storageClass", "reason"].forEach((key) =>
      nonempty(exclusion[key], `${exclusionPath}.${key}`),
    );
    boolean(exclusion.registered, `${exclusionPath}.registered`);
  });
  countReceipt(inspection.receipt, `${path}.receipt`, [
    "manifestDomains",
    "exclusions",
    "restorable",
    "migrations",
    "adapterRestorable",
    "blocked",
  ]);
}

export function assertRestorePlan(value: unknown, path: string): void {
  const plan = record(value, path);
  digest(plan.archiveSha256, `${path}.archiveSha256`);
  digest(plan.confirmationDigest, `${path}.confirmationDigest`);
  array(plan.entries, `${path}.entries`).forEach((value, index) => {
    const entryPath = `${path}.entries[${index}]`;
    const entry = record(value, entryPath);
    nonempty(entry.domainId, `${entryPath}.domainId`);
    discriminant(
      entry.choice,
      ["useArchive", "keepCurrent"],
      `${entryPath}.choice`,
    );
    nullableString(entry.action, `${entryPath}.action`);
    if (entry.current !== null) currentEvidence(entry.current, `${entryPath}.current`);
  });
  countReceipt(plan.receipt, `${path}.receipt`, [
    "selected",
    "skipped",
    "creates",
    "replaces",
    "deletes",
    "migrations",
    "unchanged",
  ]);
}

export function assertRestoreExecutionReceipt(
  value: unknown,
  path: string,
): void {
  const receipt = record(value, path);
  digest(receipt.confirmationDigest, `${path}.confirmationDigest`);
  countReceipt(receipt.staging, `${path}.staging`, [
    "selected",
    "documents",
    "deletions",
    "unchanged",
    "totalDocumentBytes",
  ]);
  publication(receipt.safetyBackup, `${path}.safetyBackup`);
  [
    "restoredDomainIds",
    "deletedDomainIds",
    "migratedDomainIds",
    "unchangedDomainIds",
    "skippedDomainIds",
    "excludedDomainIds",
  ].forEach((key) => stringArray(receipt[key], `${path}.${key}`));
}

export function assertRestoreFailure(value: unknown, path: string): void {
  const failure = record(value, path);
  nonempty(failure.stage, `${path}.stage`);
  nullableString(failure.domainId, `${path}.domainId`);
  discriminant(
    failure.terminal,
    ["noLiveMutation", "rolledBack", "recoveryRequired"],
    `${path}.terminal`,
  );
  string(failure.detail, `${path}.detail`);
}

export function assertRestoreAdapterReceipt(
  value: unknown,
  path: string,
): void {
  const receipt = record(value, path);
  ["domainId", "adapter", "outcome"].forEach((key) =>
    nonempty(receipt[key], `${path}.${key}`),
  );
  participation(receipt.participation, `${path}.participation`);
  digest(receipt.confirmationDigest, `${path}.confirmationDigest`);
  nullableDigest(receipt.evidence, `${path}.evidence`);
}

export function assertRestoreRecoveryReceipt(
  value: unknown,
  path: string,
): void {
  const receipt = record(value, path);
  discriminant(
    receipt.outcome,
    ["noRecoveryNeeded", "rolledBack", "terminalCleanup"],
    `${path}.outcome`,
  );
  stringArray(receipt.domainIds, `${path}.domainIds`);
}

export function assertPublication(value: unknown, path: string): void {
  publication(value, path);
}

function storage(value: unknown, path: string): void {
  const projection = record(value, path);
  const layout = record(projection.layout, `${path}.layout`);
  discriminant(layout.profile, STORAGE_PROFILES, `${path}.layout.profile`);
  ["platform", "canonicalApplicationId", "effectiveLeaf"].forEach((key) =>
    nonempty(layout[key], `${path}.layout.${key}`),
  );
  digest(layout.layoutDigest, `${path}.layout.layoutDigest`);
  array(layout.roots, `${path}.layout.roots`).forEach((rootValue, index) => {
    const root = record(rootValue, `${path}.layout.roots[${index}]`);
    ["kind", "path", "provenance"].forEach((key) =>
      nonempty(root[key], `${path}.layout.roots[${index}].${key}`),
    );
  });
  const bootstrap = record(projection.bootstrap, `${path}.bootstrap`);
  discriminant(bootstrap.state, STORAGE_BOOTSTRAP_STATES, `${path}.bootstrap.state`);
  array(projection.availableProfiles, `${path}.availableProfiles`).forEach(
    (profile, index) =>
      discriminant(profile, STORAGE_PROFILES, `${path}.availableProfiles[${index}]`),
  );
}

function backup(value: unknown, path: string): void {
  const projection = record(value, path);
  const inventory = record(projection.inventory, `${path}.inventory`);
  nonempty(inventory.root, `${path}.inventory.root`);
  array(inventory.archives, `${path}.inventory.archives`).forEach(
    (archiveValue, index) => {
      const archivePath = `${path}.inventory.archives[${index}]`;
      const archive = record(archiveValue, archivePath);
      ["path", "archiveId", "createdAt", "kind"].forEach((key) =>
        nonempty(archive[key], `${archivePath}.${key}`),
      );
      digest(archive.archiveSha256, `${archivePath}.archiveSha256`);
    },
  );
  array(inventory.entries, `${path}.inventory.entries`).forEach(
    (entryValue, index) =>
      inventoryEntry(entryValue, `${path}.inventory.entries[${index}]`),
  );
  boolean(inventory.complete, `${path}.inventory.complete`);
  const pending = record(projection.pending, `${path}.pending`);
  discriminant(pending.state, BACKUP_PENDING_STATES, `${path}.pending.state`);
  const encryption = record(projection.encryption, `${path}.encryption`);
  discriminant(encryption.state, BACKUP_ENCRYPTION_STATES, `${path}.encryption.state`);
  if (projection.retention !== null) {
    const retention = record(projection.retention, `${path}.retention`);
    stringArray(retention.deletionPaths, `${path}.retention.deletionPaths`);
    digest(retention.confirmationDigest, `${path}.retention.confirmationDigest`);
    array(retention.diagnostics, `${path}.retention.diagnostics`).forEach(
      (entryValue, index) =>
        inventoryEntry(entryValue, `${path}.retention.diagnostics[${index}]`),
    );
  }
}

function restoreState(value: unknown, path: string): void {
  const projection = record(value, path);
  discriminant(
    projection.state,
    ["inactive", "active", "recoveryRequired"],
    `${path}.state`,
  );
  nullableDigest(projection.safetyBackupSha256, `${path}.safetyBackupSha256`);
}

function identity(value: Record<string, unknown>, path: string): void {
  ["application", "producer"].forEach((key) => {
    const statusPath = `${path}.${key}`;
    const status = record(value[key], statusPath);
    discriminant(status.status, RESTORE_IDENTITY_STATUSES, `${statusPath}.status`);
    if (status.status === "mismatch") {
      nonempty(status.expected, `${statusPath}.expected`);
      nonempty(status.archive, `${statusPath}.archive`);
    }
  });
}

function restoreDomain(value: unknown, path: string): void {
  const domain = record(value, path);
  ["domainId", "storageClass", "consistencyGroup", "adapter", "sourceState"].forEach(
    (key) => nonempty(domain[key], `${path}.${key}`),
  );
  if (domain.sourceSchemaVersion !== null) {
    finiteNumber(domain.sourceSchemaVersion, `${path}.sourceSchemaVersion`);
  }
  if (domain.targetSchemaVersion !== null) {
    finiteNumber(domain.targetSchemaVersion, `${path}.targetSchemaVersion`);
  }
  const compatibility = record(domain.compatibility, `${path}.compatibility`);
  discriminant(
    compatibility.status,
    RESTORE_DOMAIN_COMPATIBILITY_STATUSES,
    `${path}.compatibility.status`,
  );
  if (compatibility.status === "migrationRequired") {
    finiteNumber(compatibility.from, `${path}.compatibility.from`);
    finiteNumber(compatibility.to, `${path}.compatibility.to`);
  }
  if (compatibility.status === "customAdapterReady") {
    nonempty(compatibility.adapter, `${path}.compatibility.adapter`);
    participation(compatibility.participation, `${path}.compatibility.participation`);
    digest(compatibility.confirmationDigest, `${path}.compatibility.confirmationDigest`);
  }
}

function participation(value: unknown, path: string): void {
  const projection = record(value, path);
  discriminant(
    projection.kind,
    RESTORE_ADAPTER_PARTICIPATION_KINDS,
    `${path}.kind`,
  );
  if (projection.kind === "excluded") nonempty(projection.reason, `${path}.reason`);
}

function currentEvidence(value: unknown, path: string): void {
  const evidence = record(value, path);
  discriminant(evidence.state, RESTORE_CURRENT_EVIDENCE_STATES, `${path}.state`);
  if (evidence.state === "present") {
    finiteNumber(evidence.byteLength, `${path}.byteLength`);
    digest(evidence.sha256, `${path}.sha256`);
  }
}

function inventoryEntry(value: unknown, path: string): void {
  const entry = record(value, path);
  discriminant(
    entry.state,
    ["valid", "locked", "corrupt", "foreign", "unknown", "unreadable", "unmanaged"],
    `${path}.state`,
  );
  nonempty(entry.diagnosticKind, `${path}.diagnosticKind`);
  string(entry.detail, `${path}.detail`);
}

function publication(value: unknown, path: string): void {
  const receipt = record(value, path);
  ["path", "destination", "durability"].forEach((key) =>
    nonempty(receipt[key], `${path}.${key}`),
  );
  digest(receipt.archiveSha256, `${path}.archiveSha256`);
  boolean(receipt.replacedExisting, `${path}.replacedExisting`);
}

function countReceipt(value: unknown, path: string, keys: string[]): void {
  const receipt = record(value, path);
  keys.forEach((key) => finiteNumber(receipt[key], `${path}.${key}`));
}

function stringArray(value: unknown, path: string): void {
  array(value, path).forEach((item, index) => nonempty(item, `${path}[${index}]`));
}
