<script lang="ts">
  import "@poodle/svelte-tokens/styles.css";
  import "@poodle/svelte-tokens/theme-midnight.css";

  import {
    Callout,
    PageLoading,
    UiPresentationProvider,
    createThemeController,
    type DockEdge,
  } from "@poodle/svelte";
  import type { LayoutDocument } from "@inflatable-cookie/longhorn-layout";
  import {
    LayoutDockRegion,
    createPoodleLayoutBinding,
    type PanelRenderContext,
  } from "@inflatable-cookie/longhorn-poodle";
  import {
    createPanelTransferDragSource,
    createPanelTransferDropTarget,
  } from "@inflatable-cookie/longhorn-poodle/transfer";
  import { SURFACE_TRANSFER_TARGET_KINDS } from "@inflatable-cookie/longhorn-surface-transfer";
  import type { SurfaceState } from "@inflatable-cookie/longhorn-svelte/surfaces";
  import type { SurfaceTransferState } from "@inflatable-cookie/longhorn-svelte/surface-transfer";
  import type { LayoutState } from "@inflatable-cookie/longhorn-svelte/layout";
  import type { TransferState } from "@inflatable-cookie/longhorn-svelte/transfer";
  import { onMount, tick } from "svelte";

  import { definitions, resolvePanel, schema } from "./model.ts";

  interface Props {
    layoutState: LayoutState;
    surfaceState: SurfaceState;
    transferState: TransferState;
    surfaceTransferState: SurfaceTransferState;
    loadLayoutAuthority: () => Promise<LayoutDocument>;
    reveal: () => Promise<void>;
  }

  let {
    layoutState,
    surfaceState,
    transferState,
    surfaceTransferState,
    loadLayoutAuthority,
    reveal,
  }: Props = $props();
  let hostError = $state<unknown>();
  let requestNumber = 0;
  const edges: DockEdge[] = [
    "left",
    "right",
    "left",
    "right",
    "top",
    "bottom",
    "bottom",
    "bottom",
  ];

  createThemeController({
    initial: "midnight",
    persistKey: null,
  });

  const binding = createPoodleLayoutBinding({
    state: layoutState,
    definitions,
    nextRequestId: () => `request:layout-${++requestNumber}`,
    onError: (error) => {
      hostError = error;
    },
  });
  const dragSource = createPanelTransferDragSource({
    state: transferState,
    makeStartRequest: (panelInstanceId) => ({
      protocol_version: 1,
      request_id: `request:drag-${++requestNumber}`,
      panel_instance_id: panelInstanceId,
    }),
    reportError: (error) => {
      hostError = error;
    },
  });
  const dropTarget = createPanelTransferDropTarget({
    state: transferState,
    selector: {
      kind: "explicit_zone",
      dropZoneId: "zone:secondary",
    },
    nextRequestId: () => `request:drop-${++requestNumber}`,
    reportError: (error) => {
      hostError = error;
    },
  });

  onMount(() => {
    let active = true;
    void (async () => {
      try {
        await Promise.all([
          layoutState.start(),
          surfaceState.start(),
          transferState.start(),
          surfaceTransferState.start(),
        ]);
        if (
          !active ||
          [layoutState, surfaceState, transferState, surfaceTransferState]
            .some(({ status }) => status.kind === "unsupported")
        ) {
          return;
        }
        const authoritative = await loadLayoutAuthority();
        if (!active) return;
        if (!layoutState.accept(authoritative)) {
          throw new Error("layout authority was older than the current document");
        }
        if (surfaceState.snapshot === undefined) {
          throw new Error("Surface authority was unavailable after connection");
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
        void Promise.all([
          layoutState.destroy(),
          surfaceState.destroy(),
          transferState.destroy(),
          surfaceTransferState.destroy(),
        ]);
      });
    };
  });

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  const unsupportedReason = $derived(
    [layoutState, surfaceState, transferState, surfaceTransferState]
      .map(({ status }) =>
        status.kind === "unsupported" ? status.reason : null)
      .find((reason) => reason !== null),
  );
  const reconnecting = $derived(
    surfaceState.status.kind === "reconnecting" ||
      transferState.status.kind === "reconnecting",
  );
</script>

{#snippet panelBody(context: PanelRenderContext)}
  <article data-panel-body={context.instance.id}>
    <h2>{context.presentation.label}</h2>
    <p>Loophole owns studio policy and panel content.</p>
  </article>
{/snippet}

<UiPresentationProvider density="compact" sizeScale="sm">
  {#if hostError !== undefined}
    <Callout
      tone="danger"
      title="Studio host failed"
      message={errorMessage(hostError)}
      announceMode="assertive"
    />
  {:else if unsupportedReason}
    <Callout
      tone="warning"
      title="Required capability unavailable"
      message={unsupportedReason}
      announceMode="polite"
    />
  {:else if reconnecting}
    <Callout
      tone="pending"
      title="Reconnecting workspace"
      message="Authoritative Surface and transfer state are being refreshed."
      announceMode="polite"
    />
  {:else if
    layoutState.status.kind === "idle" ||
    layoutState.status.kind === "loading" ||
    surfaceState.status.kind === "idle" ||
    surfaceState.status.kind === "loading" ||
    transferState.status.kind === "idle" ||
    transferState.status.kind === "loading"}
    <PageLoading message="Loading window, Surface, and layout authority" />
  {:else if layoutState.projected && surfaceState.snapshot}
    <main
      data-shell="loophole"
      data-display="display:studio"
      aria-label="Loophole studio"
    >
      <section data-window="window:studio">
        {#each surfaceState.snapshot.document.surfaces as surface (surface.id)}
          <article
            data-surface={surface.id}
            data-surface-transfer-targets={SURFACE_TRANSFER_TARGET_KINDS.join(",")}
          >
            <header><h1>{surface.label ?? surface.id}</h1></header>
            <div data-layout-container={surface.layout_container_id}>
              {#each schema.regions as region, index (region.id)}
                <LayoutDockRegion
                  {binding}
                  containerId={surface.layout_container_id}
                  regionId={region.id}
                  edge={edges[index]}
                  {resolvePanel}
                  ariaLabel={`${region.id} region`}
                  externalDragSource={region.id === "primary" ? dragSource : null}
                  externalDropTarget={region.id === "secondary" ? dropTarget : null}
                  body={panelBody}
                />
              {/each}
            </div>
          </article>
        {/each}
      </section>
    </main>
  {/if}
</UiPresentationProvider>
