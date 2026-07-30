<script lang="ts">
  import "@poodle/svelte-tokens/styles.css";
  import "@poodle/svelte-tokens/theme-clay.css";

  import {
    UiPresentationProvider,
    createThemeController,
  } from "@poodle/svelte";
  import {
    SettingsShell,
    type SettingsPageRenderContext,
    type SettingsSession,
  } from "@longhorn/settings/poodle";
  import { tick, untrack } from "svelte";

  let {
    session,
    reveal,
  }: {
    session: SettingsSession;
    reveal: () => Promise<void>;
  } = $props();
  let revealed = $state(false);

  createThemeController({ initial: "clay", persistKey: null });

  $effect(() => {
    if (session.status.kind === "ready" && !untrack(() => revealed)) {
      revealed = true;
      void tick().then(reveal);
    }
  });
</script>

{#snippet preferencePage(context: SettingsPageRenderContext)}
  <article data-testid="bovine-preferences" data-dirty={context.dirty}>
    <h3>Editing</h3>
    <p>One consumer-owned staged preference domain.</p>
    <button
      type="button"
      onclick={() =>
        void context.change("bovine:preferences", {
          codecVersion: 1,
          value: { compactEditor: true },
        })}
    >
      Stage compact editor
    </button>
    <button
      type="button"
      onclick={() =>
        void context.requestReset("bovine:preferences", ["bovine:primary"])}
    >
      Reset preference
    </button>
  </article>
{/snippet}

<UiPresentationProvider density="comfortable" sizeScale="md">
  <SettingsShell
    {session}
    host="modal"
    title="Bovine preferences"
    ariaLabel="Bovine preferences"
    resolveRenderer={() => preferencePage}
  />
</UiPresentationProvider>
