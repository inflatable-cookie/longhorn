<script lang="ts">
  import { UpdateStatus as PoodleUpdateStatus } from "@inflatable-cookie/poodle-svelte";
  import type { UpdateController } from "@inflatable-cookie/longhorn/update";

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

  const update = sampleUpdateController(controller);

  function install(): void {
    const version = update.actionableVersion;
    if (version !== undefined) void controller.install(version);
  }

  // "Not now", which is the operator's own decision rather than the gate's.
  // The controller keeps the two apart and only this one quietens the icon.
  function defer(): void {
    const version = update.actionableVersion;
    if (version !== undefined) void controller.defer(version, { cause: "userPostponed" });
  }
</script>

<PoodleUpdateStatus
  status={update.status}
  availability={update.availability}
  progress={update.progress}
  channel={update.channel}
  installedVersion={update.installedVersion}
  deferral={update.deferral}
  lastRejection={update.lastRejection}
  aheadOfChannel={update.aheadOfChannel}
  pending={update.pending}
  {confirmInstall}
  onCheck={() => void controller.check()}
  onInstall={update.actionableVersion === undefined ? null : install}
  onDefer={update.actionableVersion === undefined ? null : defer}
/>
