<script lang="ts">
  import {
    LayoutSplitView,
    type PoodleLayoutBinding,
  } from "../src/index.ts";

  interface Props {
    binding: PoodleLayoutBinding;
  }

  let { binding }: Props = $props();
</script>

<LayoutSplitView
  {binding}
  containerId="container:primary"
  sizingSlotId="navigation-width"
  primaryRegionId="navigation"
  secondaryRegionId="detail"
  ariaLabel="Nucleus workspace split"
>
  {#snippet primary()}Navigation{/snippet}
  {#snippet secondary()}Detail{/snippet}
</LayoutSplitView>
