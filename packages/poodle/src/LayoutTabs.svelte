<script lang="ts">
  import { Tabs } from "@poodle/svelte";
  import type {
    Orientation,
    TabActivationMode,
    TabVariant,
  } from "@poodle/svelte";
  import type { LayoutContainerId, RegionId } from "@longhorn/layout";
  import type { Snippet } from "svelte";

  import type { PoodleLayoutBinding } from "./binding.ts";
  import type {
    PanelPresentationResolver,
    PanelRenderContext,
  } from "./types.ts";

  interface Props {
    binding: PoodleLayoutBinding;
    containerId: LayoutContainerId;
    regionId: RegionId;
    resolvePanel: PanelPresentationResolver;
    ariaLabel: string;
    variant?: TabVariant;
    orientation?: Orientation;
    activationMode?: TabActivationMode;
    body?: Snippet<[PanelRenderContext]>;
  }

  let {
    binding,
    containerId,
    regionId,
    resolvePanel,
    ariaLabel,
    variant = "text",
    orientation = "horizontal",
    activationMode = "automatic",
    body,
  }: Props = $props();

  const projection = $derived(
    binding.region(containerId, regionId, resolvePanel),
  );

  function activate(panelInstanceId: string): void {
    if (panelInstanceId !== projection.state.active_panel_instance_id) {
      binding.activate(panelInstanceId);
    }
  }
</script>

{#snippet renderBody(panelInstanceId: string)}
  {@const context = projection.panels.get(panelInstanceId)}
  {#if context}
    {@render body?.(context)}
  {/if}
{/snippet}

<Tabs
  items={projection.items.map((item) => ({
    ...item,
    icon: item.icon ?? undefined,
  }))}
  value={projection.state.active_panel_instance_id ?? ""}
  {variant}
  {orientation}
  {activationMode}
  reorderable={true}
  {ariaLabel}
  onValueChange={activate}
  onClose={(panelInstanceId) => binding.close(panelInstanceId)}
  onReorder={(panelInstanceIds) =>
    binding.reorder(containerId, regionId, panelInstanceIds)}
  children={body ? renderBody : undefined}
/>
