<script lang="ts">
  import "@poodle/svelte-tokens/styles.css";
  import "@poodle/svelte-tokens/theme-graphite.css";

  import {
    Callout,
    PageLoading,
    UiPresentationProvider,
    createThemeController,
  } from "@poodle/svelte";
  import { onDestroy } from "svelte";

  type Status =
    | { kind: "loading" }
    | { kind: "ready"; authority: string }
    | { kind: "failed"; detail: string };

  interface Props {
    shape: string;
    selectedModules: readonly string[];
    status?: Status;
    onTeardown?: () => void;
  }

  let {
    shape,
    selectedModules,
    status = { kind: "loading" },
    onTeardown = () => undefined,
  }: Props = $props();

  createThemeController({ initial: "graphite", persistKey: null });
  onDestroy(onTeardown);
</script>

<UiPresentationProvider density="compact" sizeScale="sm">
  {#if status.kind === "loading"}
    <PageLoading message="Loading authoritative desktop state" />
  {:else if status.kind === "failed"}
    <Callout
      tone="danger"
      title="Desktop host unavailable"
      message={status.detail}
      announceMode="assertive"
    />
  {:else}
    <main data-composition={shape} data-authority={status.authority}>
      <h1>Desktop application</h1>
      <p>{selectedModules.length} selected Longhorn renderer packages.</p>
    </main>
  {/if}
</UiPresentationProvider>
