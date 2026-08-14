<script lang="ts">
  import type { LicenceController } from "@inflatable-cookie/longhorn/licence";
  import { LicenceActivation, LicenceSeats, LicenceStatus } from "../../src/licence/poodle.ts";

  let {
    controller,
    surface = "status",
  }: { controller: LicenceController; surface?: "status" | "seats" | "activation" } = $props();
</script>

{#if surface === "status"}
  <LicenceStatus {controller} />
{:else if surface === "seats"}
  <LicenceSeats {controller} confirmRelease={false} />
{:else}
  <LicenceActivation {controller} />
{/if}
