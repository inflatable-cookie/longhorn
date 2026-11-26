import { untrack } from "svelte";

/** Track reads from a plain controller. The owner manages its start and stop. */
export function sampleController<Controller extends { observe(callback: () => void): () => void }>(
  getController: () => Controller,
): () => Controller {
  let revision = $state(0);

  $effect(() =>
    getController().observe(() => {
      revision = untrack(() => revision) + 1;
    }),
  );

  return () => {
    void revision;
    return getController();
  };
}
