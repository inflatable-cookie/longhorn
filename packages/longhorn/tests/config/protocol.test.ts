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
    assertValidStorageTransitionInspectCommand(
      fixture.commands.inspectTransition,
    );
    assertValidStorageTransitionExecuteCommand(
      fixture.commands.executeTransition,
    );
    assertValidStorageRecoveryCommand(fixture.commands.recoverStorage);
    assertValidStorageCleanupCommand(fixture.commands.cleanupStorage);
    assertValidBackupCreateCommand(fixture.commands.createBackup);
    assertValidBackupExportCommand(fixture.commands.exportBackup);
    assertValidBackupRetentionApplyCommand(
      fixture.commands.applyRetention,
    );
    assertValidStorageTransitionInspectOutcome(
      fixture.outcomes.inspectTransition,
    );
    assertValidStorageTransitionExecuteOutcome(
      fixture.outcomes.executeTransition,
    );
    assertValidStorageRecoveryOutcome(fixture.outcomes.recoverStorage);
    assertValidStorageCleanupOutcome(fixture.outcomes.cleanupStorage);
    assertValidBackupCreateOutcome(fixture.outcomes.createBackup);
    assertValidBackupExportOutcome(fixture.outcomes.exportBackup);
    assertValidBackupRetentionApplyOutcome(
      fixture.outcomes.applyRetention,
    );
    assertValidRestoreInspectCommand(fixture.commands.inspectRestore);
    assertValidRestorePlanCommand(fixture.commands.planRestore);
    assertValidRestoreExecuteCommand(fixture.commands.executeRestore);
    assertValidRestoreAdapterExecuteCommand(
      fixture.commands.executeAdapterRestore,
    );
    assertValidRestoreRecoveryCommand(fixture.commands.recoverRestore);
    assertValidRestoreInspectOutcome(fixture.outcomes.inspectRestore);
    assertValidRestorePlanOutcome(fixture.outcomes.planRestore);
    assertValidRestoreExecuteOutcome(fixture.outcomes.executeRestore);
    assertValidRestoreAdapterExecuteOutcome(
      fixture.outcomes.executeAdapterRestore,
    );
    assertValidRestoreRecoveryOutcome(fixture.outcomes.recoverRestore);
    fixture.restoreInspectionStates.forEach((outcome) =>
      assertValidRestoreInspectOutcome(outcome),
    );
    fixture.restorePlanStates.forEach((outcome) =>
      assertValidRestorePlanOutcome(outcome),
    );
    fixture.restoreExecutionStates.forEach((outcome) =>
      assertValidRestoreExecuteOutcome(outcome),
    );
  });

  test("fails safe on future versions and unknown discriminants", () => {
    const future = structuredClone(fixture.snapshot);
    future.protocolVersion = fixture.incompatibility.futureProtocolVersion;
    incompatible(() => assertValidConfigOperationsSnapshot(future));

    const capability = structuredClone(fixture.snapshot);
    capability.capabilities[0] = fixture.incompatibility.unknownCapability;
    incompatible(() => assertValidConfigOperationsSnapshot(capability));

    const inventory = structuredClone(fixture.snapshot);
    inventory.backup!.inventory.entries[0]!.state =
      fixture.incompatibility.unknownInventoryState;
    incompatible(() => assertValidConfigOperationsSnapshot(inventory));

    const bootstrap = structuredClone(fixture.snapshot);
    (
      bootstrap.storage as unknown as {
        bootstrap: unknown;
      }
    ).bootstrap = fixture.incompatibility.unknownBootstrapState;
    incompatible(() => assertValidConfigOperationsSnapshot(bootstrap));

    incompatible(() =>
      assertValidBackupCreateOutcome(
        fixture.incompatibility.unknownOutcomeStatus,
      ),
    );

    const compatibility = structuredClone(fixture.outcomes.inspectRestore);
    if (compatibility.status !== "ready") throw new Error("fixture drift");
    (
      compatibility.inspection.domains[0] as unknown as {
        compatibility: unknown;
      }
    ).compatibility = fixture.incompatibility.unknownRestoreCompatibility;
    incompatible(() => assertValidRestoreInspectOutcome(compatibility));
  });

  test("rejects malformed confirmation and archive digests", () => {
    const request = structuredClone(fixture.commands.snapshot);
    request.requestId = "UPPERCASE";
    incompatible(() => assertValidConfigSnapshotCommand(request));

    const execute = structuredClone(fixture.commands.executeTransition);
    execute.confirmationDigest = "not-a-digest";
    incompatible(() =>
      assertValidStorageTransitionExecuteCommand(execute),
    );

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
