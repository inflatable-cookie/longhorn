<script lang="ts">
  import { DragDropProvider } from "@inflatable-cookie/poodle-svelte";
  import type { CrossWindowDragSourceBridge } from "@inflatable-cookie/poodle-core";
  import {
    LayoutDockRegion,
    type PanelPresentationResolver,
    type PoodleLayoutBinding,
  } from "../../src/poodle/index.ts";

  interface Props {
    binding: PoodleLayoutBinding;
    resolvePanel: PanelPresentationResolver;
    staticPrimary?: boolean;
    showTabs?: boolean;
    primaryCrossWindowDragSource?: CrossWindowDragSourceBridge | null;
  }

  let {
    binding,
    resolvePanel,
    staticPrimary = false,
    showTabs = true,
    primaryCrossWindowDragSource = null,
  }: Props = $props();
</script>

<DragDropProvider>
  <div>
    <LayoutDockRegion
      {binding}
      containerId="surface:primary"
      regionId="primary"
      edge="left"
      sizing={staticPrimary ? "static" : "flexible"}
      {showTabs}
      {resolvePanel}
      ariaLabel="Primary dock"
      crossWindowDragSource={primaryCrossWindowDragSource}
    >
      {#snippet body(context)}
        <div>{context.presentation.label} primary body</div>
      {/snippet}
      {#snippet panel(context)}
        <div>{context.presentation.label} static panel</div>
      {/snippet}
    </LayoutDockRegion>
    <LayoutDockRegion
      {binding}
      containerId="surface:primary"
      regionId="secondary"
      edge="right"
      {resolvePanel}
      ariaLabel="Secondary dock"
    >
      {#snippet body(context)}
        <div>{context.presentation.label} secondary body</div>
      {/snippet}
    </LayoutDockRegion>
  </div>
</DragDropProvider>
