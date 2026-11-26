import { describe, expect, test } from "bun:test";

import {
  assertValidBackupCreateOutcome,
  assertValidBackupExportOutcome,
  assertValidBackupRetentionApplyOutcome,
  assertValidConfigOperationsSnapshot,
  assertValidStorageCleanupOutcome,
  assertValidStorageRecoveryOutcome,
  assertValidStorageTransitionExecuteOutcome,
  assertValidStorageTransitionInspectOutcome,
  ConfigProtocolValidationError,
} from "../../src/config/validation.ts";
import { fixture } from "./support.ts";
import { CONFIG_FIELDS } from "../../src/config/generated/fields.ts";

/**
 * Contract 010's Boundary Validation Target on `config` — the package the
 * inventory named first for having 858 lines of validation and zero key
 * checks. A renamed field passed straight through.
 */
describe("config boundary strictness", () => {
  test("the snapshot's allowed keys come from the authority", () => {
    expect(CONFIG_FIELDS.ConfigOperationsSnapshot).toBeDefined();
    expect(CONFIG_FIELDS.ConfigOperationsSnapshot).toContain("protocolVersion");
  });

  test("rejects an unknown field", () => {
    const snapshot: Record<string, unknown> = {};
    for (const key of CONFIG_FIELDS.ConfigOperationsSnapshot) {
      snapshot[key] = null;
    }
    expect(() => assertValidConfigOperationsSnapshot({ ...snapshot, extra: 1 })).toThrow(
      /unknown field/,
    );
  });

  test("rejects a missing field", () => {
    const snapshot: Record<string, unknown> = {};
    for (const key of CONFIG_FIELDS.ConfigOperationsSnapshot.slice(1)) {
      snapshot[key] = null;
    }
    expect(() => assertValidConfigOperationsSnapshot(snapshot)).toThrow(/missing field/);
  });

  test("rejects inherited fields that would be missing on the wire", () => {
    const { protocolVersion, ...otherwiseValid } = fixture.snapshot;
    Object.setPrototypeOf(otherwiseValid, { protocolVersion });
    expect(() => assertValidConfigOperationsSnapshot(otherwiseValid)).toThrow(/missing field/);
  });

  test("rejects an unknown storage leaf provenance", () => {
    const snapshot = fixture.snapshot;
    const storage = snapshot.storage;
    expect(() =>
      assertValidConfigOperationsSnapshot({
        ...snapshot,
        storage: { ...storage, layout: { ...storage.layout, leafProvenance: "unknown" } },
      }),
    ).toThrow(ConfigProtocolValidationError);
  });

  test("omits tagged unions rather than guessing their keys", () => {
    expect("RestoreDomainCompatibilityProjection" in CONFIG_FIELDS).toBe(false);
  });
});

describe("config outcome contents", () => {
  test("checks each field of a ready storage transition", () => {
    const ready = fixture.outcomes.inspectTransition;
    for (const change of [
      { targetProfile: "unknown" },
      { domains: [null] },
      { unknownSourcePaths: [17] },
      { conflicts: [{ kind: "blocked", path: null, detail: false }] },
    ]) {
      expect(() =>
        assertValidStorageTransitionInspectOutcome({
          ...ready,
          preview: { ...ready.preview, ...change },
        }),
      ).toThrow(ConfigProtocolValidationError);
    }
  });

  test("checks successful storage and backup evidence", () => {
    const outcomes = fixture.outcomes;
    const malformed: ReadonlyArray<readonly [(value: unknown) => void, unknown]> = [
      [
        assertValidStorageTransitionExecuteOutcome,
        { ...outcomes.executeTransition, receipt: null },
      ],
      [
        assertValidStorageRecoveryOutcome,
        {
          ...outcomes.recoverStorage,
          receipt: { ...outcomes.recoverStorage.receipt, activeLayoutDigest: "bad" },
        },
      ],
      [
        assertValidStorageCleanupOutcome,
        {
          ...outcomes.cleanupStorage,
          receipt: { ...outcomes.cleanupStorage.receipt, removedPaths: [null] },
        },
      ],
      [
        assertValidBackupCreateOutcome,
        {
          ...outcomes.createBackup,
          capture: { ...outcomes.createBackup.capture, selectedDomains: -1 },
        },
      ],
      [assertValidBackupCreateOutcome, { ...outcomes.createBackup, publication: null }],
      [
        assertValidBackupExportOutcome,
        {
          ...outcomes.exportBackup,
          publication: { ...outcomes.exportBackup.publication, replacedExisting: 0 },
        },
      ],
      [
        assertValidBackupRetentionApplyOutcome,
        { ...outcomes.applyRetention, deletedPaths: [false] },
      ],
    ];
    for (const [validate, value] of malformed) {
      expect(() => validate(value)).toThrow(ConfigProtocolValidationError);
    }
  });
});
