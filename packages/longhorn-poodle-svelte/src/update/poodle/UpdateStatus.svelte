<script lang="ts">
  import { UpdateStatus as PoodleUpdateStatus } from "@inflatable-cookie/poodle-svelte";
  import type { UpdateController } from "@inflatable-cookie/longhorn/update";

  import UpdateSpecialStatus from "./UpdateSpecialStatus.svelte";
  import { sampleUpdateController } from "./UpdateSurface.svelte.ts";

  /**
   * Binds an `UpdateController` to Poodle's `UpdateStatus`.
   *
   * The information, the download, and install-and-restart, in whatever
   * container puts it there — a settings panel or the popover below. Poodle
   * renders all five availability states; this supplies them and turns the
   * three actions back into controller commands.
   */
  interface Props {
    controller: UpdateController;
    confirmInstall?: boolean;
  }

  let { controller, confirmInstall = true }: Props = $props();

  const update = sampleUpdateController(() => controller);
</script>

{#if update.special}
  <UpdateSpecialStatus
    special={update.special}
    retry={update.retryDownload}
    canRetry={update.actionableVersion !== undefined}
    pending={update.pending}
  />
{:else}
  <PoodleUpdateStatus
    status={update.status}
    {...update.availability === undefined ? {} : { availability: update.availability }}
    {...update.progress === undefined ? {} : { progress: update.progress }}
    {...update.channel === undefined ? {} : { channel: update.channel }}
    {...update.installedVersion === undefined ? {} : { installedVersion: update.installedVersion }}
    {...update.deferral === undefined ? {} : { deferral: update.deferral }}
    {...update.lastRejection === undefined ? {} : { lastRejection: update.lastRejection }}
    {...update.aheadOfChannel === undefined ? {} : { aheadOfChannel: update.aheadOfChannel }}
    pending={update.pending}
    {confirmInstall}
    onCheck={update.check}
    onInstall={update.actionableVersion === undefined ? null : update.install}
    onDefer={update.actionableVersion === undefined ? null : update.defer}
  />
{/if}
