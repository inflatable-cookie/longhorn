<script lang="ts">
  import { SplitView } from "@poodle/svelte";
  import type { SplitOrientation } from "@poodle/svelte";
  import {
    layoutRatioFromMillionths,
    layoutRatioToUnitInterval,
    type LayoutContainerId,
    type RegionId,
    type SizingSlotId,
  } from "@longhorn/layout";
  import type { Snippet } from "svelte";

  import type { PoodleLayoutBinding } from "./binding.ts";

  interface Props {
    binding: PoodleLayoutBinding;
    containerId: LayoutContainerId;
    sizingSlotId: SizingSlotId;
    orientation?: SplitOrientation;
    primaryRegionId?: RegionId | null;
    secondaryRegionId?: RegionId | null;
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
  primaryCollapsed={primaryRegion?.collapsed ?? false}
  secondaryCollapsed={secondaryRegion?.collapsed ?? false}
  showCollapsePrimary={primaryRegionId !== null}
  showCollapseSecondary={secondaryRegionId !== null}
  {ariaLabel}
  onRatioChange={setRatio}
  onPrimaryCollapsedChange={(collapsed) => {
    if (primaryRegionId) {
      binding.setCollapsed(containerId, primaryRegionId, collapsed);
    }
  }}
  onSecondaryCollapsedChange={(collapsed) => {
    if (secondaryRegionId) {
      binding.setCollapsed(containerId, secondaryRegionId, collapsed);
    }
  }}
  {primary}
  {secondary}
/>
