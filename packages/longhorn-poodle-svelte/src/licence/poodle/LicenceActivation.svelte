<script lang="ts">
  import {
    Callout,
    LicenceActivation as PoodleLicenceActivation,
  } from "@inflatable-cookie/poodle-svelte";
  import { isProbablyATypo, parseLicenceKey } from "@inflatable-cookie/longhorn/licence";
  import type {
    LicenceController,
    LicenceCredentialProjection,
  } from "@inflatable-cookie/longhorn/licence";

  import type { LicenceRejectionCode } from "@inflatable-cookie/longhorn/licence/protocol";

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

  const licence = sampleLicenceController(() => controller);

  const rejectionMessages: Record<LicenceRejectionCode, string> = {
    malformed: "Check the licence key and try again.",
    notRecognised: "The licence was not recognised. Check your details and try again.",
    noSeatsFree: "All seats are in use. Release a seat before trying again.",
    revoked: "This licence has been revoked. Use a different licence.",
    clockRefused: "Check your device’s date and time, then try again.",
    staleAuthority: "The licence state changed. Try activating again.",
    seatNotFound: "The licence seat could not be found. Try activating again.",
    unreachable: "The licence service could not be reached. Check your connection and try again.",
  };

  let failure = $state.raw<{ controller: LicenceController; message: string } | null>(null);
  let attempt = 0;

  async function activate(detail: {
    credential: LicenceCredentialProjection;
    label: string | null;
  }): Promise<void> {
    const current = controller;
    const revision = ++attempt;
    failure = null;
    const outcome = await current.activate(detail.credential, detail.label);
    if (revision !== attempt || current !== controller || outcome.status === "committed") {
      return;
    }
    failure = {
      controller: current,
      message:
        outcome.status === "rejected"
          ? rejectionMessages[outcome.code]
          : "The app could not activate this licence. Try again.",
    };
  }
</script>

{#if mode === "key"}
  <PoodleLicenceActivation
    mode="key"
    keyFormat={{ parse: parseLicenceKey, isProbablyATypo }}
    pending={licence.pending}
    {title}
    onActivate={(detail) => void activate(detail)}
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
    onActivate={(detail) => void activate(detail)}
  />
{/if}

{#if failure?.controller === controller}
  <Callout
    tone="danger"
    title="Licence activation failed"
    message={failure.message}
    announceMode="assertive"
  />
{/if}
