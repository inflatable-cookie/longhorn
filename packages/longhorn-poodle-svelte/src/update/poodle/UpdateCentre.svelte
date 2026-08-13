<script lang="ts">
  import { UpdateCenter as PoodleUpdateCenter } from "@inflatable-cookie/poodle-svelte";
  import type { UpdateController } from "@inflatable-cookie/longhorn/update";

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

  const update = sampleUpdateController(controller);

  function install(): void {
    const version = update.actionableVersion;
    if (version !== undefined) void controller.install(version);
  }

  function defer(): void {
    const version = update.actionableVersion;
    if (version !== undefined) void controller.defer(version, { cause: "userPostponed" });
  }
</script>

<PoodleUpdateCenter
  presence={update.presence}
  status={update.status}
  availability={update.availability}
  progress={update.progress}
  channel={update.channel}
  installedVersion={update.installedVersion}
  deferral={update.deferral}
  lastRejection={update.lastRejection}
  aheadOfChannel={update.aheadOfChannel}
  pending={update.pending}
  {title}
  {ariaLabel}
  {confirmInstall}
  {onOpenChange}
  onCheck={() => void controller.check()}
  onInstall={update.actionableVersion === undefined ? null : install}
  onDefer={update.actionableVersion === undefined ? null : defer}
/>
