<script lang="ts">
  import "@inflatable-cookie/poodle-core/tokens/styles.css";
  import "@inflatable-cookie/poodle-core/tokens/theme-graphite.css";

  import {
    SettingsShell,
    type SettingsPageRenderContext,
    type SettingsSession,
  } from "@inflatable-cookie/longhorn-poodle-svelte/settings/poodle";
  import {
    UiPresentationProvider,
    createThemeController,
  } from "@inflatable-cookie/poodle-svelte";
  import { tick, untrack } from "svelte";

  let {
    session,
    reveal,
  }: {
    session: SettingsSession;
    reveal: () => Promise<void>;
  } = $props();
  let revealed = $state(false);

  createThemeController({ initial: "graphite", persistKey: null });

  $effect(() => {
    if (session.status.kind === "ready" && !untrack(() => revealed)) {
      revealed = true;
      void tick().then(reveal);
    }
  });
</script>

{#snippet generalPage(_context: SettingsPageRenderContext)}
  <article data-testid="nucleus-general">
    <h3>General</h3>
    <p>No Surface or backend module was composed.</p>
  </article>
{/snippet}

<UiPresentationProvider density="compact" sizeScale="sm">
  <SettingsShell
    {session}
    host="window"
    title="Nucleus settings"
    ariaLabel="Nucleus settings"
    resolveRenderer={() => generalPage}
  />
</UiPresentationProvider>
