<script lang="ts">
  import "@inflatable-cookie/poodle-core/tokens/styles.css";
  import "@inflatable-cookie/poodle-core/tokens/theme-clay.css";

  import {
    UiPresentationProvider,
    createThemeController,
  } from "@inflatable-cookie/poodle-svelte";
  import {
    SettingsShell,
    type SettingsPageRenderContext,
    type SettingsSession,
  } from "@inflatable-cookie/longhorn-poodle-svelte/settings/poodle";
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
  <article data-testid="split-shell-preferences" data-dirty={context.dirty}>
    <h3>Editing</h3>
    <p>One consumer-owned staged preference domain.</p>
    <button
      type="button"
      onclick={() =>
        void context.change("split-shell:preferences", {
          codecVersion: 1,
          value: { compactEditor: true },
        })}
    >
      Stage compact editor
    </button>
    <button
      type="button"
      onclick={() =>
        void context.requestReset("split-shell:preferences", ["split-shell:primary"])}
    >
      Reset preference
    </button>
  </article>
{/snippet}

<UiPresentationProvider density="comfortable" sizeScale="md">
  <SettingsShell
    {session}
    host="modal"
    title="Split-shell preferences"
    ariaLabel="Split-shell preferences"
    resolveRenderer={() => preferencePage}
  />
</UiPresentationProvider>
