<script lang="ts">
  import { UpdateCenter as PoodleUpdateCenter } from "@inflatable-cookie/poodle-svelte";
  import type { UpdateController } from "@inflatable-cookie/longhorn/update";

  import { sampleUpdateController } from "./UpdateSurface.svelte.ts";

  /**
   * Binds an `UpdateController` to Poodle's `UpdateCenter`.
   *
   * The controller supplies presence for ordinary states. Target-unavailable
   * and invalid-artifact states also keep the centre visible for their message.
   */
  interface Props {
    controller: UpdateController;
    title?: string;
    ariaLabel?: string | null;
    confirmInstall?: boolean;
    onOpenChange?: ((open: boolean) => void) | null;
  }

  let {
    controller,
    title = "Update",
    ariaLabel = null,
    confirmInstall = true,
    onOpenChange = null,
  }: Props = $props();

  const update = sampleUpdateController(() => controller);
</script>

<PoodleUpdateCenter
  presence={update.presence}
  status={update.status}
  {...update.availability === undefined ? {} : { availability: update.availability }}
  {...update.progress === undefined ? {} : { progress: update.progress }}
  {...update.channel === undefined ? {} : { channel: update.channel }}
  {...update.installedVersion === undefined ? {} : { installedVersion: update.installedVersion }}
  {...update.deferral === undefined ? {} : { deferral: update.deferral }}
  {...update.lastRejection === undefined ? {} : { lastRejection: update.lastRejection }}
  lastRejectionDetail={update.lastRejectionDetail}
  {...update.aheadOfChannel === undefined ? {} : { aheadOfChannel: update.aheadOfChannel }}
  pending={update.pending}
  {title}
  {ariaLabel}
  {confirmInstall}
  {onOpenChange}
  onDownloadAgain={update.actionableVersion === undefined ? null : update.retryDownload}
  onCheck={update.check}
  onInstall={update.actionableVersion === undefined ? null : update.install}
  onDefer={update.actionableVersion === undefined ? null : update.defer}
/>
