<script lang="ts">
  import "@inflatable-cookie/poodle-svelte-tokens/styles.css";
  import "@inflatable-cookie/poodle-svelte-tokens/theme-midnight.css";

  import {
    SettingsShell,
    type SettingsPageRenderContext,
    type SettingsRendererResolver,
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
    probeHardware,
    openKeybindingEditor,
  }: {
    session: SettingsSession;
    reveal: () => Promise<void>;
    probeHardware: () => Promise<void>;
    openKeybindingEditor: () => Promise<void>;
  } = $props();
  let revealed = $state(false);

  createThemeController({ initial: "midnight", persistKey: null });

  $effect(() => {
    if (session.status.kind === "ready" && !untrack(() => revealed)) {
      revealed = true;
      void tick().then(reveal);
    }
  });

  const resolveRenderer: SettingsRendererResolver = (rendererId) => {
    switch (rendererId) {
      case "loophole:application":
        return applicationPage;
      case "loophole:appearance":
        return appearancePage;
      case "loophole:hardware":
        return hardwarePage;
      case "loophole:keybindings":
        return keybindingsPage;
      default:
        return undefined;
    }
  };
</script>

{#snippet applicationPage(context: SettingsPageRenderContext)}
  {@const snapshot = context.snapshot("loophole:preferences")}
  {@const managed = snapshot?.values.find(({ entryId }) => entryId === "loophole:managed")}
  <article data-testid="loophole-application">
    <p>Configured and effective values remain separate.</p>
    <button
      type="button"
      onclick={() =>
        void context.change("loophole:application", {
          codecVersion: 1,
          value: { telemetry: false },
        })}
    >
      Change immediately
    </button>
    <button type="button" disabled={managed?.editability !== "editable"}>
      Change managed output
    </button>
  </article>
{/snippet}

{#snippet appearancePage(context: SettingsPageRenderContext)}
  <article data-testid="loophole-appearance">
    <p>Each dirty apply unit receives a separate receipt.</p>
    <button
      type="button"
      onclick={() =>
        void context.change("loophole:appearance", {
          codecVersion: 1,
          value: { theme: "midnight" },
        })}
    >
      Stage appearance
    </button>
    <button
      type="button"
      onclick={() =>
        void context.change("loophole:studio", {
          codecVersion: 1,
          value: { meters: "dense" },
        })}
    >
      Stage studio
    </button>
  </article>
{/snippet}

{#snippet hardwarePage(_context: SettingsPageRenderContext)}
  <article data-testid="loophole-hardware">
    <p>The hardware protocol remains consumer-owned.</p>
    <button type="button" onclick={() => void probeHardware()}>
      Probe audio hardware
    </button>
  </article>
{/snippet}

{#snippet keybindingsPage(_context: SettingsPageRenderContext)}
  <article data-testid="loophole-keybindings">
    <p>Command-aware keybinding semantics remain outside this milestone.</p>
    <button type="button" onclick={() => void openKeybindingEditor()}>
      Open keybinding editor
    </button>
  </article>
{/snippet}

<UiPresentationProvider density="compact" sizeScale="sm">
  <SettingsShell
    {session}
    host="panel"
    title="Loophole settings"
    ariaLabel="Loophole settings"
    {resolveRenderer}
  />
</UiPresentationProvider>
