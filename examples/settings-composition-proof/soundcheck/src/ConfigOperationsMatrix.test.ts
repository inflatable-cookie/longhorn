import { expect, test } from "vitest";

import type {
  BackupCreateCommand,
  RestoreExecuteCommand,
  RestoreInspectCommand,
  RestorePlanCommand,
  RestoreRecoveryCommand,
  StorageTransitionInspectCommand,
} from "@inflatable-cookie/longhorn/config";

import configFixture from "./fixtures/config-protocol-v1.json";
import { ConfigProofTransport } from "./config-proof.ts";

test("keeps inspection non-mutating and backup publication explicit", async () => {
  const transport = new ConfigProofTransport();
  const client = transport.client();
  const transition = await client.inspectStorageTransition(
    configFixture.commands.inspectTransition as unknown as StorageTransitionInspectCommand,
  );
  expect(transition.status).toBe("ready");
  expect(transport.publications).toBe(0);

  const backup = await client.createBackup(
    configFixture.commands.createBackup as unknown as BackupCreateCommand,
  );
  expect(backup.status).toBe("published");
  expect(transport.publications).toBe(1);
});

test("keeps restore inspection, planning, terminals, and recovery distinct", async () => {
  const inspectionTransport = new ConfigProofTransport();
  const inspectionClient = inspectionTransport.client();
  expect(
    (
      await inspectionClient.inspectRestore(
        configFixture.commands.inspectRestore as unknown as RestoreInspectCommand,
      )
    ).status,
  ).toBe("ready");
  expect(
    (
      await inspectionClient.planRestore(
        configFixture.commands.planRestore as unknown as RestorePlanCommand,
      )
    ).status,
  ).toBe("ready");
  expect(inspectionTransport.publications).toBe(0);

  for (const terminal of [
    "succeeded",
    "rolledBack",
    "recoveryRequired",
  ] as const) {
    const transport = new ConfigProofTransport();
    transport.restoreTerminal = terminal;
    const outcome = await transport.client().executeRestore(
      configFixture.commands.executeRestore as unknown as RestoreExecuteCommand,
    );
    expect(outcome.status).toBe(terminal);
    expect(transport.publications).toBe(1);
  }

  const recovery = new ConfigProofTransport();
  const outcome = await recovery.client().recoverRestore(
    configFixture.commands.recoverRestore as unknown as RestoreRecoveryCommand,
  );
  expect(outcome.status).toBe("recovered");
  expect(recovery.publications).toBe(1);
});
