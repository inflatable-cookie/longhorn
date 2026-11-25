<script lang="ts">
  import {
    Popover,
    StatusIndicator,
    UpdateCenter as PoodleUpdateCenter,
  } from "@inflatable-cookie/poodle-svelte";
  import type { UpdateController } from "@inflatable-cookie/longhorn/update";

  import UpdateSpecialStatus from "./UpdateSpecialStatus.svelte";
  import { sampleUpdateController } from "./UpdateSurface.svelte.ts";

  /**
   * Binds an `UpdateController` to Poodle's `UpdateCenter`.
   *
   * An icon that is not there at all until there is something to act on, and
   * a popover holding the same status surface a settings panel would show.
   * `presence` decides the first part and is the controller's, not a rule this
   * file reimplements: three availability states carry a newer version that
   * this application cannot install, and an icon for any of them would offer
   * something that cannot be taken.
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

{#if update.special}
  <Popover ariaLabel={ariaLabel ?? title} {...onOpenChange === null ? {} : { onOpenChange }}>
    {#snippet trigger()}
      <StatusIndicator
        status={update.special?.kind === "unavailable" ? "info" : "warning"}
        label={ariaLabel ?? title}
      />
    {/snippet}
    <UpdateSpecialStatus
      special={update.special}
      retry={update.retryDownload}
      canRetry={update.actionableVersion !== undefined}
      pending={update.pending}
    />
  </Popover>
{:else}
  <PoodleUpdateCenter
    presence={update.presence}
    status={update.status}
    {...update.availability === undefined ? {} : { availability: update.availability }}
    {...update.progress === undefined ? {} : { progress: update.progress }}
    {...update.channel === undefined ? {} : { channel: update.channel }}
    {...update.installedVersion === undefined ? {} : { installedVersion: update.installedVersion }}
    {...update.deferral === undefined ? {} : { deferral: update.deferral }}
    {...update.lastRejection === undefined ? {} : { lastRejection: update.lastRejection }}
    {...update.aheadOfChannel === undefined ? {} : { aheadOfChannel: update.aheadOfChannel }}
    pending={update.pending}
    {title}
    {ariaLabel}
    {confirmInstall}
    {onOpenChange}
    onCheck={update.check}
    onInstall={update.actionableVersion === undefined ? null : update.install}
    onDefer={update.actionableVersion === undefined ? null : update.defer}
  />
{/if}
