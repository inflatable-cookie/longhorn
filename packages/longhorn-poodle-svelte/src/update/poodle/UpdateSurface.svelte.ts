import type { UpdateController } from "@inflatable-cookie/longhorn/update";

/**
 * The controller's reads, sampled so Svelte can see them change.
 *
 * `UpdateController` is a plain class, not a rune, so `controller.availability`
 * in a template is not a tracked dependency: nothing re-renders when the
 * authority notifies. Poodle's components bump their own counter on `observe`,
 * but they re-derive from the props they were handed — which are stale unless
 * the parent re-evaluates them.
 *
 * # Why this stays, now that Poodle's `observe` works
 *
 * It did not, for two props. `presence` was read straight in `UpdateCenter`'s
 * template and `pending` at two `disabled=` sites, both outside the
 * notify-tracked derived, so the bump never reached them -- the icon stayed
 * hidden forever. Poodle routed both on 2026-08-13, and their contract now
 * covers it.
 *
 * This stays anyway, for one structural reason rather than distrust:
 * `UpdateSettings` renders its own control. Delegation cannot help a
 * component that draws a `Select` bound to `controller.channel`, so a
 * subscription is needed there whatever the other two do. Given one of the
 * three requires it, all three use it -- one mechanism beats two, which is
 * the principle Poodle applied to their own props.
 *
 * `tests/update` asserts Poodle's `observe` is sufficient on its own, so the
 * simplification stays available and a regression on their side is visible
 * here even though nothing depends on it.
 *
 * # React hosts cannot do this at all
 *
 * Poodle reports that `observe` plus plain reads is structurally impossible in
 * React: props are by value, so the re-render `observe` triggers re-runs with
 * the values the parent last rendered. A React host needs
 * `useSyncExternalStore` over the same authority. Longhorn ships no React
 * surface, so it does not bite here -- recorded because the sampler below is
 * the Svelte shape of the same answer.
 *
 * The controller's lifecycle is deliberately not touched. Whoever constructed
 * it starts and stops it, the same division `SettingsSession` keeps: a view
 * that started the thing it renders would stop it on unmount and take the
 * state away from every other view of the same controller.
 */
export function sampleUpdateController(getController: () => UpdateController) {
  let controller = getController();
  let notify = $state(0);

  $effect(() => {
    const nextController = getController();
    if (nextController !== controller) {
      controller = nextController;
      notify += 1;
    }
    const unobserve = nextController.observe(() => {
      notify += 1;
    });
    return () => unobserve();
  });

  return {
    get status() {
      void notify;
      return controller.status;
    },
    get special() {
      void notify;
      if (controller.pending) {
        return undefined;
      }
      if (controller.lastRejection === "malformedArtifact") {
        return { kind: "malformed" as const, detail: controller.lastRejectionDetail };
      }
      if (controller.availability?.state === "unavailableForTarget") {
        return { kind: "unavailable" as const };
      }
      return undefined;
    },
    retryDownload(): void {
      const version = actionableVersion(controller);
      if (version !== undefined) {
        void controller.prepare(version);
      }
    },
    get availability() {
      void notify;
      const availability = controller.availability;
      return availability?.state === "notChecked" || availability?.state === "unavailableForTarget"
        ? undefined
        : availability;
    },
    get progress() {
      void notify;
      return controller.progress;
    },
    get channel() {
      void notify;
      return controller.channel;
    },
    get installedVersion() {
      void notify;
      return controller.installedVersion;
    },
    get deferral() {
      void notify;
      return controller.deferral;
    },
    get lastRejection() {
      void notify;
      return controller.lastRejection === "malformedArtifact"
        ? undefined
        : controller.lastRejection;
    },
    get aheadOfChannel() {
      void notify;
      return controller.aheadOfChannel;
    },
    get presence() {
      void notify;
      return controller.presence;
    },
    get pending() {
      void notify;
      return controller.pending;
    },

    /**
     * The version an install or defer would act on.
     *
     * An offer names it; so does an artifact already downloaded and waiting.
     * Absent otherwise, and the actions are absent with it — an install button
     * with no version to install is a button that cannot work.
     */
    get actionableVersion(): string | undefined {
      void notify;
      return actionableVersion(controller);
    },

    check(): void {
      void controller.check();
    },

    install(): void {
      installUpdate(controller);
    },

    defer(): void {
      deferUpdate(controller);
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
