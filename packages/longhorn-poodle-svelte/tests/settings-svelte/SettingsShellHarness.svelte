<script lang="ts">
  import {
    SettingsShell,
    type SettingsPageRenderContext,
    type SettingsSession,
  } from "../../src/settings/poodle.ts";

  let {
    session,
    missingRenderer = false,
  }: {
    session: SettingsSession;
    missingRenderer?: boolean;
  } = $props();
</script>

{#snippet page(context: SettingsPageRenderContext)}
  <div
    data-testid="consumer-page"
    data-dirty={context.dirty}
    data-busy={context.busy}
  >
    <p>{context.page.label} consumer page</p>
    <button
      type="button"
      data-testid="change"
      onclick={() =>
        void context
          .change("app:audio", {
            codecVersion: 1,
            value: { selected: "device:studio" },
          })
          .catch(() => undefined)}
    >
      Stage audio
    </button>
    <button
      type="button"
      data-testid="reset"
      onclick={() =>
        void context.requestReset("app:audio", ["audio:output"])}
    >
      Request reset
    </button>
  </div>
{/snippet}

<SettingsShell
  {session}
  resolveRenderer={() => (missingRenderer ? undefined : page)}
/>

