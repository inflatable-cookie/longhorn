<script lang="ts">
  import {
    Button,
    Callout,
    ConfirmAction,
    FormActions,
    Grid,
    Stack,
    TextInput,
  } from "@inflatable-cookie/poodle-svelte";

  import type { CommandKeymapConflict } from "@inflatable-cookie/longhorn/commands/protocol";
  import { searchCommandSettings } from "@inflatable-cookie/longhorn/commands";
  import type { CommandSettingsRecord } from "@inflatable-cookie/longhorn/commands";

  interface Props {
    records: ReadonlyArray<CommandSettingsRecord>;
    conflicts?: ReadonlyArray<CommandKeymapConflict>;
    query?: string;
    captureBindingId?: string;
    capturedLabel?: string | null;
    dirty?: boolean;
    busy?: boolean;
    onQueryChange?: (query: string) => void;
    onCapture?: (bindingId: string) => void;
    onCancelCapture?: () => void;
    onApply?: () => void;
    onCancel?: () => void;
    onReset?: () => void;
  }

  let {
    records,
    conflicts = [],
    query = "",
    captureBindingId = undefined,
    capturedLabel = null,
    dirty = false,
    busy = false,
    onQueryChange = undefined,
    onCapture = undefined,
    onCancelCapture = undefined,
    onApply = undefined,
    onCancel = undefined,
    onReset = undefined,
  }: Props = $props();

  const platformLabels = { macOs: "macOS", windows: "Windows", linux: "Linux" };

  function bindingLabel(
    record: CommandSettingsRecord,
    binding: CommandSettingsRecord["bindings"][number],
  ): string {
    const shortcut = record.shortcuts.find(({ bindingId }) => bindingId === binding.id);
    if (shortcut !== undefined) {
      return `Change ${shortcut.label}`;
    }
    return binding.platform === "any"
      ? "Change shortcut"
      : `Change shortcut for ${platformLabels[binding.platform]}`;
  }

  let visible = $derived(searchCommandSettings(records, query));
</script>

<section aria-label="Keybindings">
  <Stack gap="sm">
    <TextInput
      type="search"
      value={query}
      ariaLabel="Search keybindings"
      placeholder="Search commands"
      onValueChange={(value) => onQueryChange?.(value)}
      onClear={() => onQueryChange?.("")}
    />

    {#if conflicts.length > 0}
      <Callout tone="warning">
        {conflicts.length === 1
          ? "1 unresolved keybinding conflict"
          : `${conflicts.length} unresolved keybinding conflicts`}
      </Callout>
    {/if}

    {#if captureBindingId !== undefined}
      <Callout tone="info">
        {capturedLabel ?? "Press a shortcut"}
        <Button variant="ghost" onClick={() => onCancelCapture?.()}>Cancel capture</Button>
      </Callout>
    {/if}

    <Stack gap="md" asRole="list">
      {#each visible as record (record.id)}
        <div data-command-id={record.id} role="listitem">
          <Grid columns="minmax(0, 1fr) auto auto" gap="sm">
            <Stack direction="row" align="center">
              <Stack gap="sm">
                <strong>{record.label}</strong>
                {#if record.description}
                  <small>{record.description}</small>
                {/if}
              </Stack>
            </Stack>
            <Stack direction="row" align="center">
              <span>
                {record.shortcuts.map(({ label }) => label).join(", ") || "Unbound"}
              </span>
            </Stack>
            <Stack direction="row" gap="sm" wrap={true}>
              {#each record.bindings as binding (binding.id)}
                {@const label = bindingLabel(record, binding)}
                <Button
                  variant="secondary"
                  disabled={busy}
                  ariaLabel={`${label} for ${record.label}`}
                  onClick={() => onCapture?.(binding.id)}
                >
                  {label}
                </Button>
              {/each}
            </Stack>
          </Grid>
        </div>
      {/each}
    </Stack>

    <FormActions>
      {#if busy}
        <Button variant="ghost" disabled>Reset</Button>
      {:else}
        <ConfirmAction
          title="Reset keyboard shortcuts?"
          description="This restores the default shortcuts and clears your custom bindings."
          tone="warning"
          triggerLabel="Reset"
          confirmLabel="Reset shortcuts"
          onConfirm={() => {
            if (!busy) {
              onReset?.();
            }
          }}
        />
      {/if}
      <Button variant="secondary" disabled={!dirty || busy} onClick={() => onCancel?.()}>
        Cancel
      </Button>
      <Button variant="primary" loading={busy} disabled={!dirty} onClick={() => onApply?.()}>
        Apply
      </Button>
    </FormActions>
  </Stack>
</section>
