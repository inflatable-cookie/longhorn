<script lang="ts">
  import { Button, Callout, FormActions, Grid, Stack, TextInput } from "@inflatable-cookie/poodle-svelte";

  import type {
    CommandKeymapConflict,
  } from "@inflatable-cookie/longhorn/commands/protocol";
  import type { CommandSettingsRecord } from "@inflatable-cookie/longhorn/commands";

  interface Props {
    records: readonly CommandSettingsRecord[];
    conflicts?: readonly CommandKeymapConflict[];
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

  let visible = $derived(
    query.trim().length === 0
      ? records
      : records.filter((record) => {
          const needle = query.toLowerCase();
          return (
            record.label.toLowerCase().includes(needle) ||
            record.id.includes(needle) ||
            record.keywords.some((keyword) =>
              keyword.toLowerCase().includes(needle),
            )
          );
        }),
  );
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
        {conflicts.length} unresolved keybinding
        {conflicts.length === 1 ? " conflict" : " conflicts"}
      </Callout>
    {/if}

    {#if captureBindingId !== undefined}
      <Callout tone="info">
        {capturedLabel ?? "Press a shortcut"}
        <Button variant="ghost" onClick={() => onCancelCapture?.()}>
          Cancel capture
        </Button>
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
                <Button
                  variant="secondary"
                  disabled={busy}
                  onClick={() => onCapture?.(binding.id)}
                >
                  Change {binding.id}
                </Button>
              {/each}
            </Stack>
          </Grid>
        </div>
      {/each}
    </Stack>

    <FormActions>
      <Button variant="ghost" disabled={busy} onClick={() => onReset?.()}>
        Reset
      </Button>
      <Button
        variant="secondary"
        disabled={!dirty || busy}
        onClick={() => onCancel?.()}
      >
        Cancel
      </Button>
      <Button
        variant="primary"
        loading={busy}
        disabled={!dirty}
        onClick={() => onApply?.()}
      >
        Apply
      </Button>
    </FormActions>
  </Stack>
</section>
