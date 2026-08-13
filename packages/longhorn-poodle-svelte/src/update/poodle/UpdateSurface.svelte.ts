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
export function sampleUpdateController(controller: UpdateController) {
  let notify = $state(0);

  $effect(() => {
    const unobserve = controller.observe(() => {
      notify += 1;
    });
    return () => unobserve();
  });

  return {
    get status() { void notify; return controller.status; },
    get availability() { void notify; return controller.availability; },
    get progress() { void notify; return controller.progress; },
    get channel() { void notify; return controller.channel; },
    get installedVersion() { void notify; return controller.installedVersion; },
    get deferral() { void notify; return controller.deferral; },
    get lastRejection() { void notify; return controller.lastRejection; },
    get aheadOfChannel() { void notify; return controller.aheadOfChannel; },
    get presence() { void notify; return controller.presence; },
    get pending() { void notify; return controller.pending; },

    /**
     * The version an install or defer would act on.
     *
     * An offer names it; so does an artifact already downloaded and waiting.
     * Absent otherwise, and the actions are absent with it — an install button
     * with no version to install is a button that cannot work.
     */
    get actionableVersion(): string | undefined {
      void notify;
      const progress = controller.progress;
      if (progress?.state === "readyToInstall") return progress.version;
      const availability = controller.availability;
      return availability?.state === "offer" ? availability.version : undefined;
    },
  };
}
