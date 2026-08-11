<script lang="ts">
  import { Tabs } from "@inflatable-cookie/poodle-svelte";
  import type {
    Orientation,
    TabActivationMode,
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
    resolvePanel: PanelPresentationResolver;
    ariaLabel: string;
    variant?: TabVariant;
    /**
     * The border under the tab list. Defaults true, which is what this
     * projection rendered before Poodle flipped the Tabs default to false;
     * a consumer that wants the borderless list now has to say so.
     */
    bordered?: boolean;
    orientation?: Orientation;
    activationMode?: TabActivationMode;
    body?: Snippet<[PanelRenderContext]>;
  }

  let {
    binding,
    containerId,
    regionId,
    resolvePanel,
    ariaLabel,
    // Poodle collapsed five Tabs variants to four: the old `text` was renamed
    // `card` and the old `card` deleted, with no alias window. Same appearance,
    // new name.
    variant = "card",
    bordered = true,
    orientation = "horizontal",
    activationMode = "automatic",
    body,
  }: Props = $props();

  const projection = $derived(
    binding.region(containerId, regionId, resolvePanel),
  );

  function activate(panelInstanceId: string): void {
    if (panelInstanceId !== projection.state.active_panel_instance_id) {
      binding.activate(panelInstanceId);
    }
  }
</script>

{#snippet renderBody(panelInstanceId: string)}
  {@const context = projection.panels.get(panelInstanceId)}
  {#if context}
    {@render body?.(context)}
  {/if}
{/snippet}

<Tabs
  items={projection.items.map((item) => ({
    ...item,
    icon: item.icon ?? undefined,
  }))}
  value={projection.state.active_panel_instance_id ?? ""}
  {variant}
  {bordered}
  {orientation}
  {activationMode}
  reorderable={true}
  {ariaLabel}
  onValueChange={activate}
  onClose={(panelInstanceId) => binding.close(panelInstanceId)}
  onReorder={(panelInstanceIds) =>
    binding.reorder(containerId, regionId, panelInstanceIds)}
  children={body ? renderBody : undefined}
/>
