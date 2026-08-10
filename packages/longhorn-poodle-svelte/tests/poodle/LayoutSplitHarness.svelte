<script lang="ts">
  import {
    LayoutSplitView,
    type PoodleLayoutBinding,
  } from "../../src/poodle/index.ts";

  interface Props {
    binding: PoodleLayoutBinding;
    primaryHidden?: boolean;
  }

  let { binding, primaryHidden = false }: Props = $props();
</script>

<LayoutSplitView
  {binding}
  containerId="surface:primary"
  sizingSlotId="center-stack"
  primaryRegionId="center_bottom"
  secondaryRegionId="right_top"
  {primaryHidden}
  ariaLabel="Nucleus workspace split"
>
  {#snippet primary()}Navigation{/snippet}
  {#snippet secondary()}Detail{/snippet}
</LayoutSplitView>
