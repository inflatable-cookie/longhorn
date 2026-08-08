<script lang="ts">
  import "@poodle/svelte-tokens/styles.css";
  import "@poodle/svelte-tokens/theme-graphite.css";

  import {
    BackupSettingsPage,
    RestoreSettingsPage,
    StorageSettingsPage,
    BACKUP_SETTINGS_RENDERER_ID,
    RESTORE_SETTINGS_RENDERER_ID,
    STORAGE_SETTINGS_RENDERER_ID,
  } from "@inflatable-cookie/longhorn-config/poodle";
  import type {
    ConfigOperationsClient,
    ConfigOperationsSnapshot,
  } from "@inflatable-cookie/longhorn-config";
  import {
    SettingsShell,
    type SettingsPageRenderContext,
    type SettingsRendererResolver,
    type SettingsSession,
  } from "@inflatable-cookie/longhorn-settings/poodle";
  import {
    UiPresentationProvider,
    createThemeController,
  } from "@poodle/svelte";
  import { tick, untrack } from "svelte";

  let {
    session,
    configClient,
    configSnapshot,
    reveal,
  }: {
    session: SettingsSession;
    configClient: ConfigOperationsClient;
    configSnapshot: ConfigOperationsSnapshot;
    reveal: () => Promise<void>;
  } = $props();
  let revealed = $state(false);
  let request = 0;

  createThemeController({ initial: "graphite", persistKey: null });

  $effect(() => {
    if (session.status.kind === "ready" && !untrack(() => revealed)) {
      revealed = true;
      void tick().then(reveal);
    }
  });

  const resolveRenderer: SettingsRendererResolver = (rendererId) => {
    switch (rendererId) {
      case "soundcheck:product":
        return productPage;
      case STORAGE_SETTINGS_RENDERER_ID:
        return storagePage;
      case BACKUP_SETTINGS_RENDERER_ID:
        return backupPage;
      case RESTORE_SETTINGS_RENDERER_ID:
        return restorePage;
      default:
        return undefined;
    }
  };
</script>

{#snippet productPage(context: SettingsPageRenderContext)}
  <article data-testid="soundcheck-product">
    <h3>Audio analysis</h3>
    <button
      type="button"
      onclick={() =>
        void context.change("soundcheck:preferences", {
          codecVersion: 1,
          value: { model: "studio-v2" },
        })}
    >
      Use studio model
    </button>
  </article>
{/snippet}

{#snippet storagePage(_context: SettingsPageRenderContext)}
  <StorageSettingsPage
    client={configClient}
    initialSnapshot={configSnapshot}
    nextRequestId={() => `config:soundcheck-${++request}`}
  />
{/snippet}

{#snippet backupPage(_context: SettingsPageRenderContext)}
  <BackupSettingsPage
    client={configClient}
    initialSnapshot={configSnapshot}
    nextRequestId={() => `config:soundcheck-${++request}`}
  />
{/snippet}

{#snippet restorePage(_context: SettingsPageRenderContext)}
  <RestoreSettingsPage
    client={configClient}
    initialSnapshot={configSnapshot}
    nextRequestId={() => `config:soundcheck-${++request}`}
  />
{/snippet}

<UiPresentationProvider density="comfortable" sizeScale="md">
  <SettingsShell
    {session}
    host="window"
    title="Soundcheck settings"
    ariaLabel="Soundcheck settings"
    {resolveRenderer}
  />
</UiPresentationProvider>
