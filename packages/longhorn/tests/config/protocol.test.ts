import { describe, expect, test } from "bun:test";

import {
  ConfigProtocolValidationError,
  assertValidBackupCreateCommand,
  assertValidBackupCreateOutcome,
  assertValidBackupExportCommand,
  assertValidBackupExportOutcome,
  assertValidBackupRetentionApplyCommand,
  assertValidBackupRetentionApplyOutcome,
  assertValidConfigOperationsSnapshot,
  assertValidConfigSnapshotCommand,
  assertValidRestoreAdapterExecuteCommand,
  assertValidRestoreAdapterExecuteOutcome,
  assertValidRestoreExecuteCommand,
  assertValidRestoreExecuteOutcome,
  assertValidRestoreInspectCommand,
  assertValidRestoreInspectOutcome,
  assertValidRestorePlanCommand,
  assertValidRestorePlanOutcome,
  assertValidRestoreRecoveryCommand,
  assertValidRestoreRecoveryOutcome,
  assertValidStorageCleanupCommand,
  assertValidStorageCleanupOutcome,
  assertValidStorageRecoveryCommand,
  assertValidStorageRecoveryOutcome,
  assertValidStorageTransitionExecuteCommand,
  assertValidStorageTransitionExecuteOutcome,
  assertValidStorageTransitionInspectCommand,
  assertValidStorageTransitionInspectOutcome,
} from "../../src/config/index.ts";
import { fixture } from "./support.ts";

describe("generated config operations fixture", () => {
  test("validates every Rust-produced payload category", () => {
    assertValidConfigOperationsSnapshot(fixture.snapshot);
    assertValidConfigSnapshotCommand(fixture.commands.snapshot);
    assertValidStorageTransitionInspectCommand(fixture.commands.inspectTransition);
    assertValidStorageTransitionExecuteCommand(fixture.commands.executeTransition);
    assertValidStorageRecoveryCommand(fixture.commands.recoverStorage);
    assertValidStorageCleanupCommand(fixture.commands.cleanupStorage);
    assertValidBackupCreateCommand(fixture.commands.createBackup);
    assertValidBackupExportCommand(fixture.commands.exportBackup);
    assertValidBackupRetentionApplyCommand(fixture.commands.applyRetention);
    assertValidStorageTransitionInspectOutcome(fixture.outcomes.inspectTransition);
    assertValidStorageTransitionExecuteOutcome(fixture.outcomes.executeTransition);
    assertValidStorageRecoveryOutcome(fixture.outcomes.recoverStorage);
    assertValidStorageCleanupOutcome(fixture.outcomes.cleanupStorage);
    assertValidBackupCreateOutcome(fixture.outcomes.createBackup);
    assertValidBackupExportOutcome(fixture.outcomes.exportBackup);
    assertValidBackupRetentionApplyOutcome(fixture.outcomes.applyRetention);
    assertValidRestoreInspectCommand(fixture.commands.inspectRestore);
    assertValidRestorePlanCommand(fixture.commands.planRestore);
    assertValidRestoreExecuteCommand(fixture.commands.executeRestore);
    assertValidRestoreAdapterExecuteCommand(fixture.commands.executeAdapterRestore);
    assertValidRestoreRecoveryCommand(fixture.commands.recoverRestore);
    assertValidRestoreInspectOutcome(fixture.outcomes.inspectRestore);
    assertValidRestorePlanOutcome(fixture.outcomes.planRestore);
    assertValidRestoreExecuteOutcome(fixture.outcomes.executeRestore);
    assertValidRestoreAdapterExecuteOutcome(fixture.outcomes.executeAdapterRestore);
    assertValidRestoreRecoveryOutcome(fixture.outcomes.recoverRestore);
    fixture.restoreInspectionStates.forEach((outcome) => assertValidRestoreInspectOutcome(outcome));
    fixture.restorePlanStates.forEach((outcome) => assertValidRestorePlanOutcome(outcome));
    fixture.restoreExecutionStates.forEach((outcome) => assertValidRestoreExecuteOutcome(outcome));
  });

  test("fails safe on future versions and unknown discriminants", () => {
    const future = structuredClone(fixture.snapshot);
    future.protocolVersion = fixture.incompatibility.futureProtocolVersion;
    incompatible(() => assertValidConfigOperationsSnapshot(future));

    const capability = structuredClone(fixture.snapshot);
    capability.capabilities[0] = fixture.incompatibility.unknownCapability;
    incompatible(() => assertValidConfigOperationsSnapshot(capability));

    const inventory = structuredClone(fixture.snapshot);
    inventory.backup.inventory.entries[0]!.state = fixture.incompatibility.unknownInventoryState;
    incompatible(() => assertValidConfigOperationsSnapshot(inventory));

    const bootstrap = structuredClone(fixture.snapshot);
    (
      bootstrap.storage as unknown as {
        bootstrap: unknown;
      }
    ).bootstrap = fixture.incompatibility.unknownBootstrapState;
    incompatible(() => assertValidConfigOperationsSnapshot(bootstrap));

    incompatible(() =>
      assertValidBackupCreateOutcome(fixture.incompatibility.unknownOutcomeStatus),
    );

    const compatibility = structuredClone(fixture.outcomes.inspectRestore);
    if (compatibility.status !== "ready") {
      throw new Error("fixture drift");
    }
    (
      compatibility.inspection.domains[0] as unknown as {
        compatibility: unknown;
      }
    ).compatibility = fixture.incompatibility.unknownRestoreCompatibility;
    incompatible(() => assertValidRestoreInspectOutcome(compatibility));
  });

  test("rejects unknown config projection values and legacy casing", () => {
    const { snapshot, outcomes } = fixture;
    const { backup } = snapshot;
    const [archive] = backup.inventory.archives;
    const restore = outcomes.inspectRestore;
    if (!archive || restore.status !== "ready") {
      throw new Error("missing config projection evidence");
    }
    const { inspection } = restore;
    const [group] = inspection.consistencyGroups;
    const [domain] = inspection.domains;
    if (!group || !domain) {
      throw new Error("missing restore group or domain");
    }

    incompatible(() =>
      assertValidConfigOperationsSnapshot({
        ...snapshot,
        backup: {
          ...backup,
          inventory: { ...backup.inventory, archives: [{ ...archive, kind: "manual" }] },
        },
      }),
    );
    for (const invalid of [
      { ...inspection, kind: "custom" },
      { ...inspection, consistencyGroups: [{ ...group, mode: "coordinatedBounded" }] },
      { ...inspection, domains: [{ ...domain, sourceState: "sourcePreserved" }] },
      { ...inspection, domains: [{ ...domain, storageClass: "userConfig" }] },
    ]) {
      incompatible(() => assertValidRestoreInspectOutcome({ ...restore, inspection: invalid }));
    }
  });

  test("validates portable backup receipt evidence and rejects unrecognized values", () => {
    const { exportBackup } = fixture.outcomes;
    if (exportBackup.status !== "published") {
      throw new Error("missing portable export evidence");
    }
    expect(exportBackup.publication.durability).toBe("portableFileAndDirectorySynced");
    assertValidBackupExportOutcome(exportBackup);
    incompatible(() =>
      assertValidBackupExportOutcome({
        ...exportBackup,
        publication: { ...exportBackup.publication, durability: "synced" },
      }),
    );
  });

  test("rejects unknown restore stages, terminals, and receipt outcomes", () => {
    const {
      unknownRestoreExecutionStage,
      unknownRestoreFailureTerminal,
      unknownRestoreAdapterOutcome,
      unknownRestoreRecoveryResult,
    } = fixture.incompatibility;
    const rolledBack = fixture.restoreExecutionStates.find(
      (outcome) => outcome.status === "rolledBack",
    );
    if (!rolledBack?.failure) {
      throw new Error("missing rollback failure in config fixture");
    }
    const adapter = fixture.outcomes.executeAdapterRestore;
    const recovery = fixture.outcomes.recoverRestore;
    incompatible(() =>
      assertValidRestoreExecuteOutcome({
        ...rolledBack,
        failure: { ...rolledBack.failure, stage: unknownRestoreExecutionStage },
      }),
    );
    incompatible(() =>
      assertValidRestoreExecuteOutcome({
        ...rolledBack,
        failure: { ...rolledBack.failure, terminal: unknownRestoreFailureTerminal },
      }),
    );
    incompatible(() =>
      assertValidRestoreAdapterExecuteOutcome({
        ...adapter,
        receipt: { ...adapter.receipt, outcome: unknownRestoreAdapterOutcome },
      }),
    );
    incompatible(() =>
      assertValidRestoreRecoveryOutcome({
        ...recovery,
        receipt: { ...recovery.receipt, outcome: unknownRestoreRecoveryResult },
      }),
    );
  });

  test("rejects malformed confirmation and archive digests", () => {
    const request = structuredClone(fixture.commands.snapshot);
    request.requestId = "UPPERCASE";
    incompatible(() => assertValidConfigSnapshotCommand(request));

    const execute = structuredClone(fixture.commands.executeTransition);
    execute.confirmationDigest = "not-a-digest";
    incompatible(() => assertValidStorageTransitionExecuteCommand(execute));

    const exportCommand = structuredClone(fixture.commands.exportBackup);
    exportCommand.archiveSha256 = "A".repeat(64);
    incompatible(() => assertValidBackupExportCommand(exportCommand));

    const restore = structuredClone(fixture.commands.executeRestore);
    restore.confirmationDigest = "stale";
    incompatible(() => assertValidRestoreExecuteCommand(restore));
  });
});

function incompatible(action: () => void): void {
  expect(action).toThrow(ConfigProtocolValidationError);
}
