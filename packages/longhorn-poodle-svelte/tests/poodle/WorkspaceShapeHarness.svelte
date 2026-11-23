<script lang="ts">
  import type { DockEdge } from "@inflatable-cookie/poodle-svelte";
  import type {
  RegionId,
} from "@inflatable-cookie/longhorn/layout";

  import { LayoutDockRegion } from '../../src/poodle/index.ts';
import type { PanelPresentationResolver, PoodleLayoutBinding } from '../../src/poodle/index.ts';

  interface Props {
    binding: PoodleLayoutBinding;
    regions: ReadonlyArray<RegionId>;
    resolvePanel: PanelPresentationResolver;
  }

  let { binding, regions, resolvePanel }: Props = $props();
  const edges: Array<DockEdge> = ["left", "right", "top", "bottom"];
</script>

<div data-workspace-shape>
  {#each regions as regionId, index (regionId)}
    <LayoutDockRegion
      {binding}
      containerId="surface:primary"
      {regionId}
      edge={edges[index % edges.length] ?? "left"}
      {resolvePanel}
      ariaLabel={`${regionId} region`}
    />
  {/each}
</div>
