import type { LicenceController } from "@inflatable-cookie/longhorn/licence";

import { sampleController } from "../../bindings/sample-controller.svelte.ts";

export function sampleLicenceController(getController: () => LicenceController) {
  const read = sampleController(getController);
  return {
    get status() {
      return read().status;
    },
    get activated() {
      return read().activated;
    },
    get usable() {
      return read().usable;
    },
    get usability() {
      return read().usability;
    },
    get trustBasis() {
      return read().trustBasis;
    },
    get useUntil() {
      return read().useUntil ?? null;
    },
    get updateUntil() {
      return read().updateUntil ?? null;
    },
    get attention() {
      return read().attention;
    },
    get seats() {
      return read().seats;
    },
    get pending() {
      return read().pending;
    },
    get lastRejection() {
      return read().lastRejection;
    },
  };
}
