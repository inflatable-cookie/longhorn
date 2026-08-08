<script lang="ts">
  import {
    KeybindingSettings,
    type CommandSettingsRecord,
  } from "../../src/commands/poodle.ts";

  let {
    records,
    onCapture,
    onApply,
  }: {
    records: readonly CommandSettingsRecord[];
    onCapture: (bindingId: string) => void;
    onApply: () => void;
  } = $props();
</script>

<KeybindingSettings
  {records}
  dirty={true}
  {onCapture}
  {onApply}
/>
