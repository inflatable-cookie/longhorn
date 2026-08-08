<script lang="ts">
  import type { DockEdge } from "@poodle/svelte";
  import type { RegionId } from "@inflatable-cookie/longhorn-layout";

  import {
    LayoutDockRegion,
    type PanelPresentationResolver,
    type PoodleLayoutBinding,
  } from "../src/index.ts";

  interface Props {
    binding: PoodleLayoutBinding;
    regions: readonly RegionId[];
    resolvePanel: PanelPresentationResolver;
  }

  let { binding, regions, resolvePanel }: Props = $props();
  const edges: DockEdge[] = ["left", "right", "top", "bottom"];
</script>

<div data-workspace-shape>
  {#each regions as regionId, index (regionId)}
    <LayoutDockRegion
      {binding}
      containerId="container:primary"
      {regionId}
      edge={edges[index % edges.length]}
      {resolvePanel}
      ariaLabel={`${regionId} region`}
    />
  {/each}
</div>
