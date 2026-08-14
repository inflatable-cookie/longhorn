<script lang="ts">
  import { LicenceSeats as PoodleLicenceSeats } from "@inflatable-cookie/poodle-svelte";
  import type { LicenceController } from "@inflatable-cookie/longhorn/licence";

  import { sampleLicenceController } from "./LicenceSurface.svelte.ts";

  /**
   * Binds a `LicenceController` to Poodle's `LicenceSeats`.
   *
   * The answer to "I got a new laptop": the seats that are not this machine
   * are the ones a release frees, without a support conversation.
   *
   * Poodle's component offers `onRename`, and it is deliberately not wired.
   * The protocol has no rename command — a label is set at activation and
   * nowhere else — so wiring it would invent a capability the authority does
   * not have. If renaming earns a command it gets carded first, as the seat
   * list itself did.
   */
  interface Props {
    controller: LicenceController;
    title?: string;
    confirmRelease?: boolean;
  }

  let { controller, title = "Activated machines", confirmRelease = true }: Props = $props();

  const licence = sampleLicenceController(controller);
  let releasing = $state<string | null>(null);

  function release(detail: { machineId: string }): void {
    releasing = detail.machineId;
    void controller.releaseSeat(detail.machineId).finally(() => {
      releasing = null;
    });
  }
</script>

{#if licence.seats.length > 0}
  <!-- An empty list means the authority does not account for seats, which is
       not a state to render — showing "0 machines" would imply an accounting
       that is not happening. -->
  <PoodleLicenceSeats
    seats={licence.seats}
    pendingMachineId={releasing}
    {title}
    {confirmRelease}
    onRelease={release}
  />
{/if}
