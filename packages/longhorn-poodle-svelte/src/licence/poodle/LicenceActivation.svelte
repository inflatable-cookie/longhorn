<script lang="ts">
  import { LicenceActivation as PoodleLicenceActivation } from "@inflatable-cookie/poodle-svelte";
  import {
    isProbablyATypo,
    parseLicenceKey,
    type LicenceController,
    type LicenceCredentialProjection,
  } from "@inflatable-cookie/longhorn/licence";

  import { sampleLicenceController } from "./LicenceSurface.svelte.ts";

  /**
   * Binds a `LicenceController` to Poodle's `LicenceActivation`.
   *
   * The key format is injected rather than mirrored — Poodle's component takes
   * a `LicenceKeyFormat` and this hands it Longhorn's, which is the design
   * that lets a mistyped key fail locally without the format existing a third
   * time. The conformance fixture binds the two implementations that do exist.
   */
  interface Props {
    controller: LicenceController;
    /** Poodle's `mode`: key entry, or the account flow with file import. */
    mode?: "key" | "account";
    title?: string;
  }

  let { controller, mode = "key", title = "Activate licence" }: Props = $props();

  const licence = sampleLicenceController(controller);

  function activate(detail: {
    credential: LicenceCredentialProjection;
    label: string | null;
  }): void {
    void controller.activate(detail.credential, detail.label);
  }
</script>

{#if mode === "key"}
  <PoodleLicenceActivation
    mode="key"
    keyFormat={{ parse: parseLicenceKey, isProbablyATypo }}
    pending={licence.pending}
    {title}
    onActivate={activate}
  />
{:else}
  <PoodleLicenceActivation
    mode="account"
    accountTokenProvider={{
      // The account flow needs a browser round trip the licence host owns.
      // Card 159's licence half waits on the CredentialStore decision, and
      // until then the account route reports itself unavailable rather than
      // pretending: a button that silently does nothing is worse than one
      // that says why.
      acquire: () => Promise.reject(new Error("account sign-in is not composed yet")),
    }}
    pending={licence.pending}
    {title}
    onActivate={activate}
  />
{/if}
