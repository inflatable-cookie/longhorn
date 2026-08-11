<script lang="ts">
  import { DockRegion } from "@inflatable-cookie/poodle-svelte";
  import type {
    DockCollapsedPosture,
    DockEdge,
    DockEmphasis,
    DockExternalDragSource,
    DockExternalDropTarget,
    DockSizing,
    ControlDensity,
    ControlSize,
    PanelTabItem,
    SemanticControlSizeRole,
    TabVariant,
  } from "@inflatable-cookie/poodle-svelte";
  import type {
  RegionId,
} from "@inflatable-cookie/longhorn/layout";
import type {
  SurfaceId,
} from "@inflatable-cookie/longhorn/surfaces";
  import type { Snippet } from "svelte";

  import type { PoodleLayoutBinding } from "./binding.ts";
  import type {
    PanelPresentationResolver,
    PanelRenderContext,
  } from "./types.ts";

  interface Props {
    binding: PoodleLayoutBinding;
    containerId: SurfaceId;
    regionId: RegionId;
    edge: DockEdge;
    resolvePanel: PanelPresentationResolver;
    ariaLabel: string;
    sizing?: DockSizing;
    collapsedPosture?: DockCollapsedPosture;
    /** Forwarded to the dock strip; false when divider-level collapse pills
     * own the affordance. */
    showCollapseToggle?: boolean;
    /** Forwarded to DockRegion; false when the host renders tabs elsewhere. */
    showTabs?: boolean;
    emphasis?: DockEmphasis;
    tabVariant?: TabVariant;
    size?: ControlSize | null;
    sizeRole?: SemanticControlSizeRole;
    density?: ControlDensity | null;
    externalDragSource?: DockExternalDragSource | null;
    externalDropTarget?: DockExternalDropTarget | null;
    body?: Snippet<[PanelRenderContext]>;
    panel?: Snippet<[PanelRenderContext]>;
  }

  let {
    binding,
    containerId,
    regionId,
    edge,
    resolvePanel,
    ariaLabel,
    sizing = "flexible",
    collapsedPosture = "icon-strip",
    showCollapseToggle = true,
    showTabs = true,
    emphasis = "standard",
    // Poodle's g13-b020 deleted the strip variant: block absorbed its list
    // padding, hover and close-button margin, and the indicator moved to the
    // active-edge axis. Poodle migrated its own DockRegion the same way.
    tabVariant = "block",
    size = null,
    sizeRole = "chrome",
    density = null,
    externalDragSource = null,
    externalDropTarget = null,
    body,
    panel,
  }: Props = $props();

  const projection = $derived(
    binding.region(containerId, regionId, resolvePanel),
  );

  function activate(panelInstanceId: string): void {
    if (panelInstanceId !== projection.state.active_panel_instance_id) {
      binding.activate(panelInstanceId);
    }
  }

  function movePanel(panelInstanceId: string): void {
    if (binding.canMove(panelInstanceId, containerId, regionId)) {
      binding.move(
        panelInstanceId,
        containerId,
        regionId,
        projection.state.panel_instance_ids.length,
      );
    }
  }
</script>

{#snippet renderBody(item: PanelTabItem | null)}
  {@const context = item ? projection.panels.get(item.value) : undefined}
  {#if context}
    {@render body?.(context)}
  {/if}
{/snippet}

{#snippet renderPanel(item: PanelTabItem)}
  {@const context = projection.panels.get(item.value)}
  {#if context}
    {@render panel?.(context)}
  {/if}
{/snippet}

<DockRegion
  {edge}
  {sizing}
  collapsible={projection.definition.collapsible}
  {showCollapseToggle}
  {showTabs}
  collapsed={projection.state.collapsed ?? false}
  {collapsedPosture}
  {emphasis}
  {tabVariant}
  {size}
  {sizeRole}
  {density}
  {externalDragSource}
  {externalDropTarget}
  dragZoneId={regionId}
  items={[...projection.items]}
  value={projection.state.active_panel_instance_id}
  {ariaLabel}
  canAcceptPanel={(panelInstanceId) =>
    binding.canMove(panelInstanceId, containerId, regionId)}
  onValueChange={activate}
  onCollapsedChange={(collapsed) =>
    binding.setCollapsed(containerId, regionId, collapsed)}
  onClose={(panelInstanceId) => binding.close(panelInstanceId)}
  onReorder={(panelInstanceIds) =>
    binding.reorder(containerId, regionId, panelInstanceIds)}
  onPanelDrop={({ panel: dropped }) => movePanel(dropped.panelId)}
  children={body ? renderBody : undefined}
  panel={panel ? renderPanel : undefined}
/>
