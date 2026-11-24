import { describe, expect, test } from "bun:test";
import history from "../../../fixtures/history/protocol-v1.json";
import licence from "../../../fixtures/licence/protocol-v1.json";
import nativeContent from "../../../fixtures/native-content/protocol-v1.json";
import notifications from "../../../fixtures/notifications/protocol-v1.json";
import operation from "../../../fixtures/operation/protocol-v1.json";
import update from "../../../fixtures/update/protocol-v1.json";
import config from "../../../fixtures/config/protocol-v1.json";
import {
  assertValidHistoryChangedEvent,
  assertValidHistoryNavigationCommand,
  assertValidHistoryNavigationResult,
  assertValidHistoryPageSnapshot,
  assertValidHistorySnapshot,
} from "../src/history/validation.ts";
import {
  assertLicenceActivateCommand,
  assertLicenceChangedEvent,
  assertLicenceOutcome,
  assertLicenceSnapshot,
} from "../src/licence/validation.ts";
import {
  assertValidNativeContentChangedEvent,
  assertValidNativeContentConnectResult,
  assertValidNativeContentDesiredUpdateResult,
} from "../src/native-content/index.ts";
import {
  assertValidNotificationChangedEvent,
  assertValidNotificationMutationCommand,
  assertValidNotificationMutationResult,
  assertValidNotificationSnapshotQuery,
} from "../src/notifications/index.ts";
import {
  assertValidOperationCancellationResult,
  assertValidOperationChangedEvent,
  assertValidOperationMutationCommand,
  assertValidOperationMutationResult,
  assertValidOperationSnapshotResponse,
} from "../src/operation/index.ts";
import {
  assertUpdateChangedEvent,
  assertUpdateInstallAuthorization,
  assertUpdateOutcome,
  assertUpdateProgressEvent,
  assertUpdateSnapshot,
} from "../src/update/validation.ts";
import { assertValidConfigOperationsSnapshot } from "../src/config/index.ts";

type Case = () => void;
function cases(name: string, values: Record<string, unknown>, checks: Record<string, Case>): void {
  describe(`${name} incompatibility fixture`, () => {
    test("accounts for every generated case", () => {
      expect(Object.keys(checks).sort()).toEqual(Object.keys(values).sort());
    });
    for (const [key, check] of Object.entries(checks)) {
      test(key, check);
    }
  });
}
function c<T>(value: T, mutate: (payload: T) => void, validate: (value: unknown) => void): Case {
  return () => {
    validate(value);
    const altered = structuredClone(value);
    mutate(altered);
    expect(() => validate(altered)).toThrow();
  };
}
cases("history", history.incompatibility, {
  futureProtocolVersion: c(
    history.snapshot,
    (payload) => {
      payload.protocolVersion = history.incompatibility.futureProtocolVersion;
    },
    assertValidHistorySnapshot,
  ),
  unknownChangedKind: c(
    history.changedEvent,
    (payload) => {
      payload.kind = history.incompatibility.unknownChangedKind;
    },
    assertValidHistoryChangedEvent,
  ),
  unknownEntryPosition: c(
    history.page,
    (payload) => {
      const entry = payload.entries.at(0);
      if (!entry) {
        throw new Error("missing history entry");
      }
      entry.position = history.incompatibility.unknownEntryPosition;
    },
    assertValidHistoryPageSnapshot,
  ),
  unknownMode: c(
    history.snapshot,
    (payload) => {
      payload.summary.mode = history.incompatibility.unknownMode;
    },
    assertValidHistorySnapshot,
  ),
  unknownNavigationDirection: c(
    history.navigationResults[0]!,
    (payload) => {
      payload.receipt!.direction = history.incompatibility.unknownNavigationDirection;
    },
    assertValidHistoryNavigationResult,
  ),
  unknownNavigationStatus: c(
    history.navigationResults[0]!,
    (payload) => {
      payload.status = history.incompatibility.unknownNavigationStatus;
    },
    assertValidHistoryNavigationResult,
  ),
  unknownNavigationTarget: c(
    history.navigationCommand,
    (payload) => {
      payload.target.kind = history.incompatibility.unknownNavigationTarget;
    },
    assertValidHistoryNavigationCommand,
  ),
  unknownRejectionCode: c(
    history.navigationResults[1]!,
    (payload) => {
      payload.rejection!.code = history.incompatibility.unknownRejectionCode;
    },
    assertValidHistoryNavigationResult,
  ),
});
cases("licence", licence.incompatibility, {
  futureProtocolVersion: c(
    licence.snapshot,
    (payload) => {
      payload.protocolVersion = licence.incompatibility.futureProtocolVersion;
    },
    assertLicenceSnapshot,
  ),
  unknownChangedKind: c(
    licence.changedEvent,
    (payload) => {
      payload.kind = licence.incompatibility.unknownChangedKind;
    },
    assertLicenceChangedEvent,
  ),
  unknownCredentialKind: c(
    licence.activateCommands[0]!,
    (payload) => {
      payload.credential.kind = licence.incompatibility.unknownCredentialKind;
    },
    assertLicenceActivateCommand,
  ),
  unknownOutcomeStatus: c(
    licence.outcomes[0]!,
    (payload) => {
      payload.status = licence.incompatibility.unknownOutcomeStatus;
    },
    assertLicenceOutcome,
  ),
  unknownRejectionCode: c(
    licence.outcomes[1]!,
    (payload) => {
      payload.code = licence.incompatibility.unknownRejectionCode;
    },
    assertLicenceOutcome,
  ),
  unknownTrustBasisKind: c(
    licence.snapshot,
    (payload) => {
      payload.licence.trustBasis.kind = licence.incompatibility.unknownTrustBasisKind;
    },
    assertLicenceSnapshot,
  ),
  unknownUsabilityState: c(
    licence.snapshot,
    (payload) => {
      payload.licence.usability.state = licence.incompatibility.unknownUsabilityState;
    },
    assertLicenceSnapshot,
  ),
});
cases("native content", nativeContent.incompatibility, {
  futureProtocolVersion: c(
    nativeContent.connect,
    (payload) => {
      payload.snapshot.protocol_version = nativeContent.incompatibility.futureProtocolVersion;
    },
    assertValidNativeContentConnectResult,
  ),
  unknownChangeKind: c(
    nativeContent.proposalEvent,
    (payload) => {
      payload.change.kind = nativeContent.incompatibility.unknownChangeKind;
    },
    assertValidNativeContentChangedEvent,
  ),
  unknownMechanism: c(
    nativeContent.connect,
    (payload) => {
      payload.snapshot.desired.capabilities.mechanism =
        nativeContent.incompatibility.unknownMechanism;
    },
    assertValidNativeContentConnectResult,
  ),
  unknownRejectionCode: c(
    nativeContent.staleRevision,
    (payload) => {
      payload.rejection.code = nativeContent.incompatibility.unknownRejectionCode;
    },
    assertValidNativeContentDesiredUpdateResult,
  ),
  unknownUpdateStatus: c(
    nativeContent.desiredUpdate,
    (payload) => {
      payload.status = nativeContent.incompatibility.unknownUpdateStatus;
    },
    assertValidNativeContentDesiredUpdateResult,
  ),
});
cases("notifications", notifications.incompatibility, {
  futureProtocolVersion: c(
    notifications.snapshotQuery,
    (payload) => {
      payload.protocolVersion = notifications.incompatibility.futureProtocolVersion;
    },
    assertValidNotificationSnapshotQuery,
  ),
  unknownChangedKind: c(
    notifications.changedEvents[0]!,
    (payload) => {
      payload.kind = notifications.incompatibility.unknownChangedKind;
    },
    assertValidNotificationChangedEvent,
  ),
  unknownMutationKind: c(
    notifications.mutationCommands[0]!,
    (payload) => {
      payload.kind = notifications.incompatibility.unknownMutationKind;
    },
    assertValidNotificationMutationCommand,
  ),
  unknownMutationStatus: c(
    notifications.mutationResults[0]!,
    (payload) => {
      payload.status = notifications.incompatibility.unknownMutationStatus;
    },
    assertValidNotificationMutationResult,
  ),
  unknownRejectionCode: c(
    notifications.mutationResults[3]!,
    (payload) => {
      payload.rejection!.code = notifications.incompatibility.unknownRejectionCode;
    },
    assertValidNotificationMutationResult,
  ),
  unknownSeverity: c(
    notifications.mutationCommands[0]!,
    (payload) => {
      payload.draft!.severity = notifications.incompatibility.unknownSeverity;
    },
    assertValidNotificationMutationCommand,
  ),
});
cases("operation", operation.incompatibility, {
  futureProtocolVersion: c(
    operation.snapshotResponse,
    (payload) => {
      payload.snapshot.protocolVersion = operation.incompatibility.futureProtocolVersion;
    },
    assertValidOperationSnapshotResponse,
  ),
  unknownCancellationStatus: c(
    operation.cancellationResult,
    (payload) => {
      payload.status = operation.incompatibility.unknownCancellationStatus;
    },
    assertValidOperationCancellationResult,
  ),
  unknownChangedKind: c(
    operation.changedEvents[0]!,
    (payload) => {
      payload.kind = operation.incompatibility.unknownChangedKind;
    },
    assertValidOperationChangedEvent,
  ),
  unknownMutationKind: c(
    operation.mutationCommands[0]!,
    (payload) => {
      payload.kind = operation.incompatibility.unknownMutationKind;
    },
    assertValidOperationMutationCommand,
  ),
  unknownMutationStatus: c(
    operation.mutationResults[0]!,
    (payload) => {
      payload.status = operation.incompatibility.unknownMutationStatus;
    },
    assertValidOperationMutationResult,
  ),
  unknownRejectionCode: c(
    operation.mutationResults[3]!,
    (payload) => {
      payload.rejection!.code = operation.incompatibility.unknownRejectionCode;
    },
    assertValidOperationMutationResult,
  ),
  unknownState: c(
    operation.cancellationResult,
    (payload) => {
      const active = payload.snapshot.active.at(0);
      if (!active) {
        throw new Error("missing active operation");
      }
      active.state = operation.incompatibility.unknownState;
    },
    assertValidOperationCancellationResult,
  ),
});
cases("update", update.incompatibility, {
  futureProtocolVersion: c(
    update.snapshot,
    (payload) => {
      payload.protocolVersion = update.incompatibility.futureProtocolVersion;
    },
    assertUpdateSnapshot,
  ),
  unknownAvailabilityState: c(
    update.snapshot,
    (payload) => {
      payload.availability.state = update.incompatibility.unknownAvailabilityState;
    },
    assertUpdateSnapshot,
  ),
  unknownChangedKind: c(
    update.changedEvent,
    (payload) => {
      payload.kind = update.incompatibility.unknownChangedKind;
    },
    assertUpdateChangedEvent,
  ),
  unknownChannel: c(
    update.snapshot,
    (payload) => {
      payload.channel = update.incompatibility.unknownChannel;
    },
    assertUpdateSnapshot,
  ),
  unknownDeferralCause: c(
    update.authorizationDeferred,
    (payload) => {
      payload.cause.cause = update.incompatibility.unknownDeferralCause;
    },
    assertUpdateInstallAuthorization,
  ),
  unknownInstallAuthorizationStatus: c(
    update.authorizationHeld,
    (payload) => {
      payload.status = update.incompatibility.unknownInstallAuthorizationStatus;
    },
    assertUpdateInstallAuthorization,
  ),
  unknownInstallManager: c(
    update.managedSnapshot,
    (payload) => {
      payload.availability.manager = update.incompatibility.unknownInstallManager;
    },
    assertUpdateSnapshot,
  ),
  unknownOfferReason: c(
    update.snapshot,
    (payload) => {
      payload.availability.reason = update.incompatibility.unknownOfferReason;
    },
    assertUpdateSnapshot,
  ),
  unknownOutcomeStatus: c(
    update.outcomes[0]!,
    (payload) => {
      payload.status = update.incompatibility.unknownOutcomeStatus;
    },
    assertUpdateOutcome,
  ),
  unknownProgressState: c(
    update.progressEvent,
    (payload) => {
      payload.progress.state = update.incompatibility.unknownProgressState;
    },
    assertUpdateProgressEvent,
  ),
  unknownRejectionCode: c(
    update.outcomes[1]!,
    (payload) => {
      payload.code = update.incompatibility.unknownRejectionCode;
    },
    assertUpdateOutcome,
  ),
});
test("accepts every generated config inventory state", () => {
  expect(config.inventoryStates.map((entry) => entry.state).sort()).toEqual(
    ["locked", "corrupt", "foreign", "unknown", "unreadable", "unmanaged", "valid"].sort(),
  );
  for (const entry of config.inventoryStates) {
    const snapshot = structuredClone(config.snapshot);
    snapshot.backup.inventory.entries = [entry];
    assertValidConfigOperationsSnapshot(snapshot);
    const invalid = structuredClone(snapshot);
    const inventory = {
      ...invalid.backup.inventory,
      entries: [{ ...entry, state: "futureState" }],
    };
    expect(() =>
      assertValidConfigOperationsSnapshot({ ...invalid, backup: { ...invalid.backup, inventory } }),
    ).toThrow();
  }
});
