<script lang="ts">
  import { SplitView } from "@poodle/svelte";
  import type {
    ControlDensity,
    ControlSize,
    SemanticControlSizeRole,
    SplitOrientation,
    SplitToggleVisibility,
  } from "@poodle/svelte";
  import {
    layoutRatioFromMillionths,
    layoutRatioToUnitInterval,
    type LayoutContainerId,
    type RegionId,
    type SizingSlotId,
  } from "@inflatable-cookie/longhorn-layout";
  import type { Snippet } from "svelte";

  import type { PoodleLayoutBinding } from "./binding.ts";

  interface Props {
    binding: PoodleLayoutBinding;
    containerId: LayoutContainerId;
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
  const primaryRegion = $derived(
    primaryRegionId
      ? binding.collapsibleRegionState(containerId, primaryRegionId)
      : null,
  );
  const secondaryRegion = $derived(
    secondaryRegionId
      ? binding.collapsibleRegionState(containerId, secondaryRegionId)
      : null,
  );

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
  primaryCollapsed={primaryHidden || (primaryRegion?.collapsed ?? false)}
  secondaryCollapsed={secondaryHidden || (secondaryRegion?.collapsed ?? false)}
  showCollapsePrimary={!primaryHidden && primaryRegionId !== null}
  showCollapseSecondary={!secondaryHidden && secondaryRegionId !== null}
  primaryCollapsedSize={primaryHidden ? 0 : null}
  secondaryCollapsedSize={secondaryHidden ? 0 : null}
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
