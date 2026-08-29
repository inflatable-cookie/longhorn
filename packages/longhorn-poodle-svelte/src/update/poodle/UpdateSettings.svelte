<script lang="ts">
  import { Field, Select, type SelectOption } from "@inflatable-cookie/poodle-svelte";
  import { UPDATE_CHANNELS, type Channel, type UpdateController } from "@inflatable-cookie/longhorn/update";

  import { sampleUpdateController } from "./UpdateSurface.svelte.ts";
  import UpdateStatus from "./UpdateStatus.svelte";

  /**
   * The update settings panel: which channel this install follows, and the
   * same status surface the popover shows.
   *
   * A composition, not a component. It owns no state and no rendering of its
   * own beyond arranging two things that already exist, which is why it lives
   * here rather than being written again in every application.
   */
  interface Props {
    controller: UpdateController;
    confirmInstall?: boolean;
  }

  let { controller, confirmInstall = true }: Props = $props();

  const update = sampleUpdateController(() => controller);

  const CHANNEL_LABELS: Record<Channel, string> = {
    production: "Production",
    beta: "Beta",
    nightly: "Nightly",
  };

  // Said out loud, because "nightly" is a word people choose without knowing
  // what they are agreeing to. Selecting a faster channel is the one setting
  // here that can leave an install ahead of what it follows.
  const CHANNEL_HINTS: Record<Channel, string> = {
    production: "Released builds. The default.",
    beta: "Release candidates, ahead of production.",
    nightly: "Continuous builds. Expect breakage.",
  };

  const options = $derived<SelectOption[]>(
    UPDATE_CHANNELS.map((value) => ({ value, label: CHANNEL_LABELS[value] })),
  );

  const selected = $derived(update.channel);
</script>

<div class="longhorn-update-settings">
  <Field
    id="longhorn-update-channel"
    label="Release channel"
    description={selected === undefined ? undefined : CHANNEL_HINTS[selected]}
  >
    <Select
      id="longhorn-update-channel"
      options={options}
      value={selected ?? null}
      disabled={update.pending || selected === undefined}
      ariaLabel="Release channel"
      onValueChange={(value) => void controller.selectChannel(value as Channel)}
    />
  </Field>

  <UpdateStatus {controller} {confirmInstall} />
</div>

<style>
  .longhorn-update-settings {
    display: grid;
    gap: var(--poodle-space-lg, 1rem);
  }
</style>
