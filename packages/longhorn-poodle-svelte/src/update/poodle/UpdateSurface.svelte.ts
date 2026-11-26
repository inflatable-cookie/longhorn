import type { UpdateController } from "@inflatable-cookie/longhorn/update";

import { sampleController } from "../../bindings/sample-controller.svelte.ts";

export function sampleUpdateController(getController: () => UpdateController) {
  const read = sampleController(getController);
  return {
    get status() {
      return read().status;
    },
    retryDownload(): void {
      const controller = getController();
      const version = actionableVersion(controller);
      if (version !== undefined) {
        void controller.prepare(version);
      }
    },
    get availability() {
      const controller = read();
      const availability = controller.availability;
      return availability?.state === "notChecked" ? undefined : availability;
    },
    get progress() {
      return read().progress;
    },
    get channel() {
      return read().channel;
    },
    get installedVersion() {
      return read().installedVersion;
    },
    get deferral() {
      return read().deferral;
    },
    get lastRejection() {
      return read().lastRejection;
    },
    get lastRejectionDetail() {
      return read().lastRejectionDetail;
    },
    get aheadOfChannel() {
      return read().aheadOfChannel;
    },
    get presence() {
      const controller = read();
      return controller.availability?.state === "unavailableForTarget" ||
        controller.lastRejection === "malformedArtifact"
        ? "quiet"
        : controller.presence;
    },
    get pending() {
      return read().pending;
    },

    /**
     * The version an install or defer would act on.
     *
     * An offer names it; so does an artifact already downloaded and waiting.
     * Absent otherwise, and the actions are absent with it — an install button
     * with no version to install is a button that cannot work.
     */
    get actionableVersion(): string | undefined {
      const controller = read();
      return actionableVersion(controller);
    },

    check(): void {
      void getController().check();
    },

    install(): void {
      installUpdate(getController());
    },

    defer(): void {
      deferUpdate(getController());
    },
  };
}

/**
 * The version an install or defer acts on, read from the controller itself.
 *
 * A retained artifact names its version through `progress`; an offer names it
 * through `availability`. Neither present means there is nothing to act on.
 */
export function actionableVersion(controller: UpdateController): string | undefined {
  const progress = controller.progress;
  if (progress?.state === "readyToInstall") {
    return progress.version;
  }
  const availability = controller.availability;
  return availability?.state === "offer" ? availability.version : undefined;
}

/**
 * Install, which is two halves of one button.
 *
 * A retained artifact is applied; anything else is staged first, and the
 * surface's progress then says when to restart. This lives beside the sampled
 * projection because both Poodle bindings offer the same button: the update
 * centre's popover and the settings status panel act on one controller, and the
 * stage-apply decision must not be made in two places.
 */
export function installUpdate(controller: UpdateController): void {
  const version = actionableVersion(controller);
  if (version === undefined) {
    return;
  }
  if (controller.progress?.state === "readyToInstall") {
    void controller.apply(version);
  } else {
    void controller.prepare(version);
  }
}

/**
 * "Not now" — the operator's own decision rather than the gate's.
 *
 * The controller keeps the two apart and only this one quietens the icon. Also
 * shared, for the same reason as `installUpdate`.
 */
export function deferUpdate(controller: UpdateController): void {
  const version = actionableVersion(controller);
  if (version !== undefined) {
    void controller.defer(version, { cause: "userPostponed" });
  }
}
