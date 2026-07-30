<script lang="ts">
  import "@poodle/svelte-tokens/styles.css";
  import "@poodle/svelte-tokens/theme-graphite.css";

  import {
    SettingsShell,
    type SettingsPageRenderContext,
    type SettingsSession,
  } from "@longhorn/settings/poodle";
  import {
    UiPresentationProvider,
    createThemeController,
  } from "@poodle/svelte";
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
