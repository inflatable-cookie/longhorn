<script lang="ts">
  import { LicenceStatus as PoodleLicenceStatus } from "@inflatable-cookie/poodle-svelte";
  import type { LicenceController } from "@inflatable-cookie/longhorn/licence";

  import { sampleLicenceController } from "./LicenceSurface.svelte.ts";

  /**
   * Binds a `LicenceController` to Poodle's `LicenceStatus`.
   *
   * Rendered only when a licence is held: unlicensed is the absence of a
   * licence rather than a usability state, and the activation surface is what
   * an unlicensed operator should be looking at.
   */
  interface Props {
    controller: LicenceController;
    title?: string;
  }

  let { controller, title = "Licence" }: Props = $props();

  const licence = sampleLicenceController(() => controller);
</script>

{#if licence.activated && licence.usability !== undefined && licence.trustBasis !== undefined}
  <PoodleLicenceStatus
    usability={licence.usability}
    trustBasis={licence.trustBasis}
    useUntil={licence.useUntil}
    updateUntil={licence.updateUntil}
    usable={licence.usable}
    attention={licence.attention}
    {title}
  />
{/if}
