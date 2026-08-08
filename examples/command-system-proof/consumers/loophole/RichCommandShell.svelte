<script lang="ts">
  import {
    CommandPaletteBinding,
    KeybindingSettings,
    type CommandSession,
  } from "@inflatable-cookie/longhorn-poodle-svelte/commands/poodle";

  let { session }: { session: CommandSession } = $props();
</script>

<button type="button" onclick={() => session.setOpen(true)}>
  Open commands
</button>

<CommandPaletteBinding
  {session}
  title="Loophole commands"
  categoryLabel={(path) => path.join(" / ")}
/>

<KeybindingSettings
  records={session.settingsRecords}
  conflicts={session.projection?.conflicts ?? []}
/>
