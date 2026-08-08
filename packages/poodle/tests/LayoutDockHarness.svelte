<script lang="ts">
  import {
    LayoutDockRegion,
    type PanelPresentationResolver,
    type PoodleLayoutBinding,
  } from "../src/index.ts";
  import type {
    DockExternalDragSource,
    DockExternalDropTarget,
  } from "@inflatable-cookie/poodle-svelte";

  interface Props {
    binding: PoodleLayoutBinding;
    resolvePanel: PanelPresentationResolver;
    staticPrimary?: boolean;
    primaryExternalDragSource?: DockExternalDragSource | null;
    secondaryExternalDropTarget?: DockExternalDropTarget | null;
  }

  let {
    binding,
    resolvePanel,
    staticPrimary = false,
    primaryExternalDragSource = null,
    secondaryExternalDropTarget = null,
  }: Props = $props();
</script>

<div>
  <LayoutDockRegion
    {binding}
    containerId="container:primary"
    regionId="primary"
    edge="left"
    sizing={staticPrimary ? "static" : "flexible"}
    {resolvePanel}
    ariaLabel="Primary dock"
    externalDragSource={primaryExternalDragSource}
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
    containerId="container:primary"
    regionId="secondary"
    edge="right"
    {resolvePanel}
    ariaLabel="Secondary dock"
    externalDropTarget={secondaryExternalDropTarget}
  >
    {#snippet body(context)}
      <div>{context.presentation.label} secondary body</div>
    {/snippet}
  </LayoutDockRegion>
</div>
