<script lang="ts">
  import "@inflatable-cookie/poodle-core/tokens/styles.css";
  import "@inflatable-cookie/poodle-core/tokens/theme-clay.css";

  import {
    Callout,
    PageLoading,
    SplitView,
    UiPresentationProvider,
    createThemeController,
  } from "@inflatable-cookie/poodle-svelte";
  import {
    type ReactiveClientState,
    useClientState,
  } from "@inflatable-cookie/longhorn-poodle-svelte";
  import { untrack } from "svelte";

  export interface BovineAuthority {
    readonly documentTitle: string;
    readonly sectionTitle: string;
  }

  interface Props {
    clientState: ReactiveClientState<BovineAuthority>;
    reveal: () => Promise<void>;
    reportError?: (error: unknown) => void;
  }

  let { clientState, reveal, reportError = () => undefined }: Props = $props();
  let revealed = $state(false);

  createThemeController({
    initial: "clay",
    persistKey: null,
  });
  useClientState(clientState, reportError);

  $effect(() => {
    const snapshot = clientState.snapshot;
    if (
      clientState.status.kind === "ready" &&
      snapshot !== undefined &&
      !untrack(() => revealed)
    ) {
      revealed = true;
      void reveal().catch(reportError);
    }
  });

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }
</script>

<UiPresentationProvider density="comfortable" sizeScale="md">
  {#if clientState.status.kind === "idle" || clientState.status.kind === "loading"}
    <PageLoading message="Loading workspace authority" />
  {:else if clientState.status.kind === "unsupported"}
    <Callout
      tone="warning"
      title="Host capability unavailable"
      message={clientState.status.reason}
      announceMode="polite"
    />
  {:else if clientState.status.kind === "reconnecting"}
    <Callout
      tone="pending"
      title="Reconnecting"
      message="Keeping the last authoritative document visible."
      announceMode="polite"
    />
  {:else if clientState.status.kind === "failed"}
    <Callout
      tone="danger"
      title="Workspace failed"
      message={errorMessage(clientState.status.error)}
      announceMode="assertive"
    />
  {:else if clientState.snapshot}
    <main data-shell="bovine" aria-label="Bovine workspace">
      <h1>{clientState.snapshot.documentTitle}</h1>
      {#snippet navigation()}
        <nav aria-label="Sections">
          <button type="button">{clientState.snapshot!.sectionTitle}</button>
        </nav>
      {/snippet}
      {#snippet document()}
        <article aria-label="Document">
          <h2>Draft</h2>
          <p>Product-owned content stays outside Longhorn.</p>
        </article>
      {/snippet}
      <SplitView
        ariaLabel="Navigation and document"
        ratio={0.28}
        primary={navigation}
        secondary={document}
      />
    </main>
  {/if}
</UiPresentationProvider>
