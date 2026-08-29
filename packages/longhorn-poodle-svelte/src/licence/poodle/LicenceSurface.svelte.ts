import type { LicenceController } from "@inflatable-cookie/longhorn/licence";

/**
 * The controller's reads, sampled so Svelte can see them change.
 *
 * The same shape as `sampleUpdateController` and for the same measured reason:
 * `LicenceController` is a plain class, so its getters are not tracked
 * dependencies, and Poodle's components re-derive from the props they were
 * handed. The bump has to happen on the side that owns the expressions.
 *
 * The lifecycle is deliberately not touched. Whoever constructed the
 * controller starts and stops it.
 */
export function sampleLicenceController(getController: () => LicenceController) {
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
    get status() { void notify; return controller.status; },
    get activated() { void notify; return controller.activated; },
    get usable() { void notify; return controller.usable; },
    get usability() { void notify; return controller.usability; },
    get trustBasis() { void notify; return controller.trustBasis; },
    get useUntil() { void notify; return controller.useUntil ?? null; },
    get updateUntil() { void notify; return controller.updateUntil ?? null; },
    get attention() { void notify; return controller.attention; },
    get seats() { void notify; return controller.seats; },
    get pending() { void notify; return controller.pending; },
    get lastRejection() { void notify; return controller.lastRejection; },
  };
}
