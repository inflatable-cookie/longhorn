<script lang="ts">
  import type { UpdateController } from "@inflatable-cookie/longhorn/update";
  import { UpdateCentre, UpdateStatus } from "../../src/update/poodle.ts";

  let { controller, centre = false }: { controller: UpdateController; centre?: boolean } =
    $props();
</script>

{#if centre}
  <UpdateCentre {controller} />
{:else}
  <UpdateStatus {controller} />
{/if}
