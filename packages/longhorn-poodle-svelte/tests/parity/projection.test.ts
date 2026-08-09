import { describe, expect, it } from "vitest";

import fixture from "../../../../fixtures/parity/projection-v1.json";

import {
  notificationStatusTone,
} from "../../src/notifications/poodle/projectors.ts";
import {
  canCancelOperation,
  operationProgressView,
  operationStateLabel,
  operationStatusTone,
} from "../../src/operation/poodle/projectors.ts";
import {
  canUseArchive,
  compatibilityLabel,
} from "../../src/config/poodle/restore-model.ts";

/**
 * The other half of `crates/longhorn-poodle/tests/parity.rs`.
 *
 * Both sides answer to `fixtures/parity/projection-v1.json`, so a mapping
 * changed in one tier and not the other fails a gate rather than waiting to be
 * spotted by eye. Nobody generates the fixture: wording is generated because
 * one source should decide it (card 170), but behaviour is not, because the
 * value here is two independent implementations agreeing. See card 171.
 */
describe("cross-backend projection parity", () => {
  it("maps every notification severity to the tone the fixture states", () => {
    expect(fixture.notificationSeverityTone).toHaveLength(5);

    for (const testCase of fixture.notificationSeverityTone) {
      expect(
        notificationStatusTone(testCase.severity as never),
        JSON.stringify(testCase),
      ).toBe(testCase.tone);
    }
  });

  it("maps every operation state to the tone and label the fixture states", () => {
    expect(fixture.operationState).toHaveLength(7);

    for (const testCase of fixture.operationState) {
      const state = testCase.state as never;
      expect(operationStatusTone(state), JSON.stringify(testCase)).toBe(
        testCase.tone,
      );
      expect(operationStateLabel(state), JSON.stringify(testCase)).toBe(
        testCase.label,
      );
    }
  });

  it("turns every progress shape into the bar the fixture states", () => {
    for (const testCase of fixture.operationProgress) {
      const view = operationProgressView(testCase.progress as never);

      expect(view.indeterminate, JSON.stringify(testCase)).toBe(
        testCase.indeterminate,
      );
      expect(view.value, JSON.stringify(testCase)).toBe(testCase.value);
      expect(view.max, JSON.stringify(testCase)).toBe(testCase.max);
      expect(view.valueText, JSON.stringify(testCase)).toBe(testCase.valueText);
    }
  });

  it("offers cancellation exactly where the fixture states", () => {
    for (const testCase of fixture.cancelEligibility) {
      const entry = {
        cancellationSupport: testCase.support,
        state: testCase.state,
      } as never;

      expect(canCancelOperation(entry), JSON.stringify(testCase)).toBe(
        testCase.canCancel,
      );
    }
  });

  it("labels and gates every restore classification as the fixture states", () => {
    expect(fixture.restoreCompatibility).toHaveLength(13);

    for (const testCase of fixture.restoreCompatibility) {
      const compatibility = testCase.compatibility as never;

      expect(compatibilityLabel(compatibility), JSON.stringify(testCase)).toBe(
        testCase.label,
      );
      expect(
        canUseArchive({ compatibility } as never),
        JSON.stringify(testCase),
      ).toBe(testCase.canUseArchive);
    }
  });

  it("records the differences that are deliberate rather than omitting them", () => {
    // A parity suite that only lists agreements reads as though there are no
    // differences. Every entry here is a decision or an open question from
    // memo 022, and each carries its own reason.
    expect(fixture.deliberateDifferences.length).toBeGreaterThan(0);
    for (const difference of fixture.deliberateDifferences) {
      expect(difference.what).toBeTruthy();
      expect(difference.why).toBeTruthy();
    }
  });
});
