<script lang="ts">
  import {
    LayoutSplitView,
    type PoodleLayoutBinding,
  } from "../../src/poodle/index.ts";

  interface Props {
    binding: PoodleLayoutBinding;
    primaryHidden?: boolean;
    primaryRegionId?: string;
    secondaryRegionId?: string;
  }

  let {
    binding,
    primaryHidden = false,
    primaryRegionId = "center_bottom",
    secondaryRegionId = "right_top",
  }: Props = $props();
</script>

<LayoutSplitView
  {binding}
  containerId="surface:primary"
  sizingSlotId="center-stack"
  {primaryRegionId}
  {secondaryRegionId}
  {primaryHidden}
  ariaLabel="Nucleus workspace split"
>
  {#snippet primary()}Navigation{/snippet}
  {#snippet secondary()}Detail{/snippet}
</LayoutSplitView>
