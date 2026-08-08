<script lang="ts">
  import type { LayoutContainerId, RegionId } from "@inflatable-cookie/longhorn/layout";

  import {
    LayoutTabs,
    type PanelPresentationResolver,
    type PoodleLayoutBinding,
  } from "../../src/poodle/index.ts";

  interface Props {
    binding: PoodleLayoutBinding;
    containerId: LayoutContainerId;
    regionId: RegionId;
    resolvePanel: PanelPresentationResolver;
  }

  let { binding, containerId, regionId, resolvePanel }: Props = $props();
</script>

<LayoutTabs
  {binding}
  {containerId}
  {regionId}
  {resolvePanel}
  ariaLabel="Workspace tabs"
>
  {#snippet body(context)}
    <div data-panel-body={context.instance.id}>
      {context.presentation.label} body
    </div>
  {/snippet}
</LayoutTabs>
