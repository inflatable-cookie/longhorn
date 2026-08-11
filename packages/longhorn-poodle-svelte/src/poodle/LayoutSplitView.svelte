<script lang="ts">
  import { SplitView } from "@inflatable-cookie/poodle-svelte";
  import type {
    ControlDensity,
    ControlSize,
    SemanticControlSizeRole,
    SplitOrientation,
  } from "@inflatable-cookie/poodle-svelte";
  import {
    layoutRatioFromMillionths,
    layoutRatioToUnitInterval,
    type SurfaceId,
    type RegionId,
    type SizingSlotId,
  } from "@inflatable-cookie/longhorn/layout";
  import type { ComponentProps, Snippet } from "svelte";

  import type { PoodleLayoutBinding } from "./binding.ts";

  // Poodle 0.1.0 defines SplitToggleVisibility but does not re-export it from
  // its package root, and contract 012 forbids reaching past that root.
  // Derive it from the public component instead of mirroring the union.
  type SplitToggleVisibility = NonNullable<
    ComponentProps<typeof SplitView>["toggleVisibility"]
  >;

  interface Props {
    binding: PoodleLayoutBinding;
    containerId: SurfaceId;
    sizingSlotId: SizingSlotId;
    orientation?: SplitOrientation;
    primaryRegionId?: RegionId | null;
    secondaryRegionId?: RegionId | null;
    primaryHidden?: boolean;
    secondaryHidden?: boolean;
    /** Forwarded to SplitView; "hover" hides the collapse pills until the
     * divider is hovered or focused. */
    toggleVisibility?: SplitToggleVisibility;
    size?: ControlSize | null;
    sizeRole?: SemanticControlSizeRole;
    density?: ControlDensity | null;
    ariaLabel: string;
    primary?: Snippet<[]>;
    secondary?: Snippet<[]>;
  }

  let {
    binding,
    containerId,
    sizingSlotId,
    orientation = "horizontal",
    primaryRegionId = null,
    secondaryRegionId = null,
    primaryHidden = false,
    secondaryHidden = false,
    toggleVisibility = "always",
    size = null,
    sizeRole = "chrome",
    density = null,
    ariaLabel,
    primary,
    secondary,
  }: Props = $props();

  const sizing = $derived(binding.sizingSlot(containerId, sizingSlotId));
  // Non-collapsible regions are valid split members; they just have no pill.
  // collapsibleRegionState throws for those — probe the definition first.
  const primaryRegion = $derived.by(() => {
    if (!primaryRegionId) return null;
    const definition = binding.regionDefinition(containerId, primaryRegionId);
    if (!definition.collapsible) return null;
    return binding.collapsibleRegionState(containerId, primaryRegionId);
  });
  const secondaryRegion = $derived.by(() => {
    if (!secondaryRegionId) return null;
    const definition = binding.regionDefinition(containerId, secondaryRegionId);
    if (!definition.collapsible) return null;
    return binding.collapsibleRegionState(containerId, secondaryRegionId);
  });

  function setRatio(ratio: number): void {
    binding.setSizingSlot(
      containerId,
      sizingSlotId,
      layoutRatioFromMillionths(Math.round(ratio * 1_000_000)),
    );
  }
</script>

<SplitView
  {orientation}
  ratio={layoutRatioToUnitInterval(sizing.ratio)}
  minRatio={layoutRatioToUnitInterval(sizing.definition.minimum)}
  maxRatio={layoutRatioToUnitInterval(sizing.definition.maximum)}
  primaryCollapsed={primaryRegion?.collapsed ?? false}
  secondaryCollapsed={secondaryRegion?.collapsed ?? false}
  primaryHidden={primaryHidden}
  secondaryHidden={secondaryHidden}
  showCollapsePrimary={!primaryHidden && primaryRegion !== null}
  showCollapseSecondary={!secondaryHidden && secondaryRegion !== null}
  disabled={primaryHidden || secondaryHidden}
  {size}
  {sizeRole}
  {density}
  {toggleVisibility}
  {ariaLabel}
  onRatioChange={setRatio}
  onPrimaryCollapsedChange={(collapsed) => {
    if (!primaryHidden && primaryRegionId) {
      binding.setCollapsed(containerId, primaryRegionId, collapsed);
    }
  }}
  onSecondaryCollapsedChange={(collapsed) => {
    if (!secondaryHidden && secondaryRegionId) {
      binding.setCollapsed(containerId, secondaryRegionId, collapsed);
    }
  }}
  {primary}
  {secondary}
/>
