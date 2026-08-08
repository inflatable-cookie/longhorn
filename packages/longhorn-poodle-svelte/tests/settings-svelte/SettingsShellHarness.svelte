<script lang="ts">
  import {
    SettingsShell,
    type SettingsHostForm,
    type SettingsPageRenderContext,
    type SettingsSession,
  } from "../../src/settings/poodle.ts";

  let {
    session,
    host = "window",
    missingRenderer = false,
  }: {
    session: SettingsSession;
    host?: SettingsHostForm;
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
  {host}
  resolveRenderer={() => (missingRenderer ? undefined : page)}
/>

