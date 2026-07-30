import { describe, expect, test } from "bun:test";

import {
  ConfigProtocolIncompatibilityError,
  assertCompatibleBackupCreateCommand,
  assertCompatibleBackupCreateOutcome,
  assertCompatibleBackupExportCommand,
  assertCompatibleBackupExportOutcome,
  assertCompatibleBackupRetentionApplyCommand,
  assertCompatibleBackupRetentionApplyOutcome,
  assertCompatibleConfigOperationsSnapshot,
  assertCompatibleConfigSnapshotCommand,
  assertCompatibleRestoreAdapterExecuteCommand,
  assertCompatibleRestoreAdapterExecuteOutcome,
  assertCompatibleRestoreExecuteCommand,
  assertCompatibleRestoreExecuteOutcome,
  assertCompatibleRestoreInspectCommand,
  assertCompatibleRestoreInspectOutcome,
  assertCompatibleRestorePlanCommand,
  assertCompatibleRestorePlanOutcome,
  assertCompatibleRestoreRecoveryCommand,
  assertCompatibleRestoreRecoveryOutcome,
  assertCompatibleStorageCleanupCommand,
  assertCompatibleStorageCleanupOutcome,
  assertCompatibleStorageRecoveryCommand,
  assertCompatibleStorageRecoveryOutcome,
  assertCompatibleStorageTransitionExecuteCommand,
  assertCompatibleStorageTransitionExecuteOutcome,
  assertCompatibleStorageTransitionInspectCommand,
  assertCompatibleStorageTransitionInspectOutcome,
} from "../src/index.ts";
import { fixture } from "./support.ts";

describe("generated config operations fixture", () => {
  test("validates every Rust-produced payload category", () => {
    assertCompatibleConfigOperationsSnapshot(fixture.snapshot);
    assertCompatibleConfigSnapshotCommand(fixture.commands.snapshot);
    assertCompatibleStorageTransitionInspectCommand(
      fixture.commands.inspectTransition,
    );
    assertCompatibleStorageTransitionExecuteCommand(
      fixture.commands.executeTransition,
    );
    assertCompatibleStorageRecoveryCommand(fixture.commands.recoverStorage);
    assertCompatibleStorageCleanupCommand(fixture.commands.cleanupStorage);
    assertCompatibleBackupCreateCommand(fixture.commands.createBackup);
    assertCompatibleBackupExportCommand(fixture.commands.exportBackup);
    assertCompatibleBackupRetentionApplyCommand(
      fixture.commands.applyRetention,
    );
    assertCompatibleStorageTransitionInspectOutcome(
      fixture.outcomes.inspectTransition,
    );
    assertCompatibleStorageTransitionExecuteOutcome(
      fixture.outcomes.executeTransition,
    );
    assertCompatibleStorageRecoveryOutcome(fixture.outcomes.recoverStorage);
    assertCompatibleStorageCleanupOutcome(fixture.outcomes.cleanupStorage);
    assertCompatibleBackupCreateOutcome(fixture.outcomes.createBackup);
    assertCompatibleBackupExportOutcome(fixture.outcomes.exportBackup);
    assertCompatibleBackupRetentionApplyOutcome(
      fixture.outcomes.applyRetention,
    );
    assertCompatibleRestoreInspectCommand(fixture.commands.inspectRestore);
    assertCompatibleRestorePlanCommand(fixture.commands.planRestore);
    assertCompatibleRestoreExecuteCommand(fixture.commands.executeRestore);
    assertCompatibleRestoreAdapterExecuteCommand(
      fixture.commands.executeAdapterRestore,
    );
    assertCompatibleRestoreRecoveryCommand(fixture.commands.recoverRestore);
    assertCompatibleRestoreInspectOutcome(fixture.outcomes.inspectRestore);
    assertCompatibleRestorePlanOutcome(fixture.outcomes.planRestore);
    assertCompatibleRestoreExecuteOutcome(fixture.outcomes.executeRestore);
    assertCompatibleRestoreAdapterExecuteOutcome(
      fixture.outcomes.executeAdapterRestore,
    );
    assertCompatibleRestoreRecoveryOutcome(fixture.outcomes.recoverRestore);
    fixture.restoreInspectionStates.forEach((outcome) =>
      assertCompatibleRestoreInspectOutcome(outcome),
    );
    fixture.restorePlanStates.forEach((outcome) =>
      assertCompatibleRestorePlanOutcome(outcome),
    );
    fixture.restoreExecutionStates.forEach((outcome) =>
      assertCompatibleRestoreExecuteOutcome(outcome),
    );
  });

  test("fails safe on future versions and unknown discriminants", () => {
    const future = structuredClone(fixture.snapshot);
    future.protocolVersion = fixture.incompatibility.futureProtocolVersion;
    incompatible(() => assertCompatibleConfigOperationsSnapshot(future));

    const capability = structuredClone(fixture.snapshot);
    capability.capabilities[0] = fixture.incompatibility.unknownCapability;
    incompatible(() => assertCompatibleConfigOperationsSnapshot(capability));

    const inventory = structuredClone(fixture.snapshot);
    inventory.backup!.inventory.entries[0]!.state =
      fixture.incompatibility.unknownInventoryState;
    incompatible(() => assertCompatibleConfigOperationsSnapshot(inventory));

    const bootstrap = structuredClone(fixture.snapshot);
    (
      bootstrap.storage as unknown as {
        bootstrap: unknown;
      }
    ).bootstrap = fixture.incompatibility.unknownBootstrapState;
    incompatible(() => assertCompatibleConfigOperationsSnapshot(bootstrap));

    incompatible(() =>
      assertCompatibleBackupCreateOutcome(
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
    incompatible(() => assertCompatibleRestoreInspectOutcome(compatibility));
  });

  test("rejects malformed confirmation and archive digests", () => {
    const request = structuredClone(fixture.commands.snapshot);
    request.requestId = "UPPERCASE";
    incompatible(() => assertCompatibleConfigSnapshotCommand(request));

    const execute = structuredClone(fixture.commands.executeTransition);
    execute.confirmationDigest = "not-a-digest";
    incompatible(() =>
      assertCompatibleStorageTransitionExecuteCommand(execute),
    );

    const exportCommand = structuredClone(fixture.commands.exportBackup);
    exportCommand.archiveSha256 = "A".repeat(64);
    incompatible(() => assertCompatibleBackupExportCommand(exportCommand));

    const restore = structuredClone(fixture.commands.executeRestore);
    restore.confirmationDigest = "stale";
    incompatible(() => assertCompatibleRestoreExecuteCommand(restore));
  });
});

function incompatible(action: () => void): void {
  expect(action).toThrow(ConfigProtocolIncompatibilityError);
}
