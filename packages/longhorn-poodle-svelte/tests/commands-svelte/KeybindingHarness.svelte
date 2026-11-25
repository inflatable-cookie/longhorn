<script lang="ts">
  import type { CommandKeymapConflict } from "@inflatable-cookie/longhorn/commands/protocol";
  import { KeybindingSettings } from "../../src/commands/poodle.ts";
  import type { CommandSettingsRecord } from "../../src/commands/poodle.ts";

  let {
    records,
    conflicts = [],
    onCapture,
    onApply,
  }: {
    records: ReadonlyArray<CommandSettingsRecord>;
    conflicts?: ReadonlyArray<CommandKeymapConflict>;
    onCapture: (bindingId: string) => void;
    onApply: () => void;
  } = $props();
</script>

<KeybindingSettings {records} {conflicts} dirty={true} {onCapture} {onApply} />
