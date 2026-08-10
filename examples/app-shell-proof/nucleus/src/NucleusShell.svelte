<script lang="ts">
  import "@inflatable-cookie/poodle-core/tokens/styles.css";
  import "@inflatable-cookie/poodle-core/tokens/theme-graphite.css";

  import {
    Callout,
    PageLoading,
    UiPresentationProvider,
    createThemeController,
    type DockEdge,
  } from "@inflatable-cookie/poodle-svelte";
  import type {
  SurfaceDocument,
} from "@inflatable-cookie/longhorn/surfaces";
  import {
    LayoutDockRegion,
    createPoodleLayoutBinding,
    type PanelRenderContext,
  } from "@inflatable-cookie/longhorn-poodle-svelte/poodle";
  import { onMount, tick } from "svelte";
  import type { LayoutState } from "@inflatable-cookie/longhorn-poodle-svelte/layout";

  import { definitions, resolvePanel, schema } from "./model.ts";

  interface Props {
    layoutState: LayoutState;
    loadAuthority: () => Promise<SurfaceDocument>;
    reveal: () => Promise<void>;
  }

  let { layoutState, loadAuthority, reveal }: Props = $props();
  let hostError = $state<unknown>();
  let requestNumber = 0;
  const edges: DockEdge[] = ["left", "top", "right", "bottom", "bottom"];

  createThemeController({
    initial: "graphite",
    persistKey: null,
  });

  const binding = createPoodleLayoutBinding({
    state: layoutState,
    definitions,
    nextRequestId: () => `request:nucleus-${++requestNumber}`,
    onError: (error) => {
      hostError = error;
    },
  });

  onMount(() => {
    let active = true;
    void (async () => {
      try {
        await layoutState.start();
        if (!active || layoutState.status.kind === "unsupported") return;
        const authoritative = await loadAuthority();
        if (!active) return;
        if (!layoutState.accept(authoritative)) {
          throw new Error("layout authority was older than the current document");
        }
        await tick();
        await reveal();
      } catch (error) {
        if (active) hostError = error;
      }
    })();
    return () => {
      active = false;
      queueMicrotask(() => {
        void layoutState.destroy();
      });
    };
  });

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }
</script>

{#snippet panelBody(context: PanelRenderContext)}
  <article data-panel-body={context.instance.id}>
    <h2>{context.presentation.label}</h2>
    <p>Nucleus owns project and agent presentation.</p>
  </article>
{/snippet}

<UiPresentationProvider density="compact" sizeScale="sm">
  {#if hostError !== undefined}
    <Callout
      tone="danger"
      title="Workspace host failed"
      message={errorMessage(hostError)}
      announceMode="assertive"
    />
  {:else if layoutState.status.kind === "idle" || layoutState.status.kind === "loading"}
    <PageLoading message="Loading registered layout" />
  {:else if layoutState.status.kind === "unsupported"}
    <Callout
      tone="warning"
      title="Layout capability unavailable"
      message={layoutState.status.reason}
      announceMode="polite"
    />
  {:else if layoutState.status.kind === "reconnecting"}
    <Callout
      tone="pending"
      title="Reconnecting layout"
      message="The last authoritative layout remains visible."
      announceMode="polite"
    />
  {:else if layoutState.projected}
    <main data-shell="nucleus" data-window="workspace:main" aria-label="Nucleus workspace">
      <header><h1>Nucleus</h1></header>
      <div data-layout-container="surface:nucleus">
        {#each schema.regions as region, index (region.id)}
          <LayoutDockRegion
            {binding}
            containerId="surface:nucleus"
            regionId={region.id}
            edge={edges[index]}
            {resolvePanel}
            ariaLabel={`${region.id} region`}
            body={panelBody}
          />
        {/each}
      </div>
    </main>
  {/if}
</UiPresentationProvider>
