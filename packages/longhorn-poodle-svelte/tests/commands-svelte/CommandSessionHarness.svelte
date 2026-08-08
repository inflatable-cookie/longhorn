<script lang="ts">
  import {
    CommandPaletteBinding,
    useCommandSession,
    type CommandSession,
  } from "../../src/commands/poodle.ts";

  let { session }: { session: CommandSession } = $props();
  useCommandSession(session);
</script>

<button type="button" onclick={() => session.setOpen(true)}>
  Open commands
</button>
<CommandPaletteBinding {session} />
