import { describe, expect, test } from "bun:test";
import history from "../../../fixtures/history/protocol-v1.json";
import licence from "../../../fixtures/licence/protocol-v1.json";
import nativeContent from "../../../fixtures/native-content/protocol-v1.json";
import notifications from "../../../fixtures/notifications/protocol-v1.json";
import operation from "../../../fixtures/operation/protocol-v1.json";
import update from "../../../fixtures/update/protocol-v1.json";
import config from "../../../fixtures/config/protocol-v1.json";
import * as h from "../src/history/validation.ts";
import * as l from "../src/licence/validation.ts";
import * as n from "../src/native-content/index.ts";
import * as nt from "../src/notifications/index.ts";
import * as o from "../src/operation/index.ts";
import * as u from "../src/update/validation.ts";
import { assertValidConfigOperationsSnapshot } from "../src/config/index.ts";

type Case = { value: unknown; path: (string | number)[]; validate: (value: unknown) => void };
function cases(name: string, values: Record<string, unknown>, checks: Record<string, Case>): void {
  describe(`${name} incompatibility fixture`, () => {
    test("accounts for every generated case", () => {
      expect(Object.keys(checks).sort()).toEqual(Object.keys(values).sort());
    });
    for (const [key, check] of Object.entries(checks)) {
      test(key, () => {
        check.validate(check.value);
        const altered: unknown = structuredClone(check.value);
        let parent: unknown = altered;
        for (const part of check.path.slice(0, -1)) {
          if (parent === null || typeof parent !== "object" || !(part in parent))
            throw new Error(`missing path for ${key}`);
          parent = Reflect.get(parent, part);
        }
        const last = check.path.at(-1);
        if (
          last === undefined ||
          parent === null ||
          typeof parent !== "object" ||
          !(last in parent)
        )
          throw new Error(`missing path for ${key}`);
        Reflect.set(parent, last, values[key]);
        expect(() => check.validate(altered)).toThrow();
      });
    }
  });
}
const c = (
  value: unknown,
  path: (string | number)[],
  validate: (value: unknown) => void,
): Case => ({ value, path, validate });
cases("history", history.incompatibility, {
  futureProtocolVersion: c(history.snapshot, ["protocolVersion"], h.assertValidHistorySnapshot),
  unknownChangedKind: c(history.changedEvent, ["kind"], h.assertValidHistoryChangedEvent),
  unknownEntryPosition: c(
    history.page,
    ["entries", 0, "position"],
    h.assertValidHistoryPageSnapshot,
  ),
  unknownMode: c(history.snapshot, ["summary", "mode"], h.assertValidHistorySnapshot),
  unknownNavigationDirection: c(
    history.navigationResults[0],
    ["receipt", "direction"],
    h.assertValidHistoryNavigationResult,
  ),
  unknownNavigationStatus: c(
    history.navigationResults[0],
    ["status"],
    h.assertValidHistoryNavigationResult,
  ),
  unknownNavigationTarget: c(
    history.navigationCommand,
    ["target", "kind"],
    h.assertValidHistoryNavigationCommand,
  ),
  unknownRejectionCode: c(
    history.navigationResults[1],
    ["rejection", "code"],
    h.assertValidHistoryNavigationResult,
  ),
});
cases("licence", licence.incompatibility, {
  futureProtocolVersion: c(licence.snapshot, ["protocolVersion"], l.assertLicenceSnapshot),
  unknownChangedKind: c(licence.changedEvent, ["kind"], l.assertLicenceChangedEvent),
  unknownCredentialKind: c(
    licence.activateCommands[0],
    ["credential", "kind"],
    l.assertLicenceActivateCommand,
  ),
  unknownOutcomeStatus: c(licence.outcomes[0], ["status"], l.assertLicenceOutcome),
  unknownRejectionCode: c(licence.outcomes[1], ["code"], l.assertLicenceOutcome),
  unknownTrustBasisKind: c(
    licence.snapshot,
    ["licence", "trustBasis", "kind"],
    l.assertLicenceSnapshot,
  ),
  unknownUsabilityState: c(
    licence.snapshot,
    ["licence", "usability", "state"],
    l.assertLicenceSnapshot,
  ),
});
cases("native content", nativeContent.incompatibility, {
  futureProtocolVersion: c(
    nativeContent.connect,
    ["snapshot", "protocol_version"],
    n.assertValidNativeContentConnectResult,
  ),
  unknownChangeKind: c(
    nativeContent.proposalEvent,
    ["change", "kind"],
    n.assertValidNativeContentChangedEvent,
  ),
  unknownMechanism: c(
    nativeContent.connect,
    ["snapshot", "desired", "capabilities", "mechanism"],
    n.assertValidNativeContentConnectResult,
  ),
  unknownRejectionCode: c(
    nativeContent.staleRevision,
    ["rejection", "code"],
    n.assertValidNativeContentDesiredUpdateResult,
  ),
  unknownUpdateStatus: c(
    nativeContent.desiredUpdate,
    ["status"],
    n.assertValidNativeContentDesiredUpdateResult,
  ),
});
cases("notifications", notifications.incompatibility, {
  futureProtocolVersion: c(
    notifications.snapshotQuery,
    ["protocolVersion"],
    nt.assertValidNotificationSnapshotQuery,
  ),
  unknownChangedKind: c(
    notifications.changedEvents[0],
    ["kind"],
    nt.assertValidNotificationChangedEvent,
  ),
  unknownMutationKind: c(
    notifications.mutationCommands[0],
    ["kind"],
    nt.assertValidNotificationMutationCommand,
  ),
  unknownMutationStatus: c(
    notifications.mutationResults[0],
    ["status"],
    nt.assertValidNotificationMutationResult,
  ),
  unknownRejectionCode: c(
    notifications.mutationResults[3],
    ["rejection", "code"],
    nt.assertValidNotificationMutationResult,
  ),
  unknownSeverity: c(
    notifications.mutationCommands[0],
    ["draft", "severity"],
    nt.assertValidNotificationMutationCommand,
  ),
});
cases("operation", operation.incompatibility, {
  futureProtocolVersion: c(
    operation.snapshotResponse,
    ["snapshot", "protocolVersion"],
    o.assertValidOperationSnapshotResponse,
  ),
  unknownCancellationStatus: c(
    operation.cancellationResult,
    ["status"],
    o.assertValidOperationCancellationResult,
  ),
  unknownChangedKind: c(operation.changedEvents[0], ["kind"], o.assertValidOperationChangedEvent),
  unknownMutationKind: c(
    operation.mutationCommands[0],
    ["kind"],
    o.assertValidOperationMutationCommand,
  ),
  unknownMutationStatus: c(
    operation.mutationResults[0],
    ["status"],
    o.assertValidOperationMutationResult,
  ),
  unknownRejectionCode: c(
    operation.mutationResults[3],
    ["rejection", "code"],
    o.assertValidOperationMutationResult,
  ),
  unknownState: c(
    operation.cancellationResult,
    ["snapshot", "active", 0, "state"],
    o.assertValidOperationCancellationResult,
  ),
});
cases("update", update.incompatibility, {
  futureProtocolVersion: c(update.snapshot, ["protocolVersion"], u.assertUpdateSnapshot),
  unknownAvailabilityState: c(update.snapshot, ["availability", "state"], u.assertUpdateSnapshot),
  unknownChangedKind: c(update.changedEvent, ["kind"], u.assertUpdateChangedEvent),
  unknownChannel: c(update.snapshot, ["channel"], u.assertUpdateSnapshot),
  unknownDeferralCause: c(
    update.authorizationDeferred,
    ["cause", "cause"],
    u.assertUpdateInstallAuthorization,
  ),
  unknownInstallAuthorizationStatus: c(
    update.authorizationHeld,
    ["status"],
    u.assertUpdateInstallAuthorization,
  ),
  unknownInstallManager: c(
    update.managedSnapshot,
    ["availability", "manager"],
    u.assertUpdateSnapshot,
  ),
  unknownOfferReason: c(update.snapshot, ["availability", "reason"], u.assertUpdateSnapshot),
  unknownOutcomeStatus: c(update.outcomes[0], ["status"], u.assertUpdateOutcome),
  unknownProgressState: c(update.progressEvent, ["progress", "state"], u.assertUpdateProgressEvent),
  unknownRejectionCode: c(update.outcomes[1], ["code"], u.assertUpdateOutcome),
});
test("accepts every generated config inventory state", () => {
  for (const entry of config.inventoryStates) {
    const snapshot = structuredClone(config.snapshot);
    snapshot.backup.inventory.entries = [entry];
    assertValidConfigOperationsSnapshot(snapshot);
  }
});
