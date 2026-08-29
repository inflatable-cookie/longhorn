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
   * `onRename` is wired since 2026-08-14: the protocol gained
   * `LicenceRenameSeatCommand`, so the capability the component offered now
   * exists. The label stays the customer's word, and clearing it renames to
   * null rather than to an empty string.
   */
  interface Props {
    controller: LicenceController;
    title?: string;
    confirmRelease?: boolean;
  }

  let { controller, title = "Activated machines", confirmRelease = true }: Props = $props();

  const licence = sampleLicenceController(() => controller);
  let releasing = $state<string | null>(null);

  function rename(detail: { machineId: string; label: string | null }): void {
    // Poodle's editor can hand back an empty string for a cleared field; the
    // protocol treats empty as a mistake and null as "unnamed", so the
    // normalisation happens here, at the seam.
    const label = detail.label !== null && detail.label.trim().length === 0 ? null : detail.label;
    void controller.renameSeat(detail.machineId, label);
  }

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
    onRename={rename}
  />
{/if}
