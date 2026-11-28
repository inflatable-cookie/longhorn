<script lang="ts">
  import {
    Button,
    Callout,
    ConfirmAction,
    EmptyState,
    FormActions,
    Grid,
    Spinner,
    Stack,
    TextInput,
  } from "@inflatable-cookie/poodle-svelte";

  import type { CommandKeymapConflict } from "@inflatable-cookie/longhorn/commands/protocol";
  import { searchCommandSettings } from "@inflatable-cookie/longhorn/commands";
  import type {
    CommandControllerStatus,
    CommandMutationStatus,
    CommandSettingsRecord,
  } from "@inflatable-cookie/longhorn/commands";
  import { errorMessage } from "../../failures.ts";

  interface Props {
    records: ReadonlyArray<CommandSettingsRecord>;
    status?: CommandControllerStatus;
    mutation?: CommandMutationStatus;
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
    status = { kind: "ready" },
    mutation = undefined,
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
  const saving = $derived(
    busy || mutation?.kind === "previewing" || mutation?.kind === "committing",
  );
</script>

<section aria-label="Keybindings">
  <Stack gap="sm">
    {#if status.kind === "idle" || status.kind === "loading"}
      <Spinner ariaLabel="Loading keyboard shortcuts" />
    {:else if status.kind === "failed"}
      <Callout
        tone="danger"
        title="Keyboard shortcuts could not be loaded"
        message={errorMessage(status.error)}
        announceMode="polite"
      />
    {:else if status.kind === "unavailable" || status.kind === "recovery"}
      <Callout
        tone="warning"
        title={status.kind === "recovery"
          ? "Keyboard shortcuts need recovery"
          : "Keyboard shortcuts unavailable"}
        message={status.kind === "recovery" ? status.recovery.detail : status.detail}
        announceMode="polite"
      />
    {:else}
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

      {#if mutation?.kind === "failed"}
        <Callout
          tone="danger"
          title="Keyboard shortcuts could not be saved"
          message={errorMessage(mutation.error)}
          announceMode="polite"
        />
      {:else if mutation?.kind === "conflict"}
        <Callout
          tone="warning"
          title="Keyboard shortcut changes conflict"
          message={mutation.detail}
          announceMode="polite"
        />
      {:else if mutation?.kind === "saved"}
        <Callout tone="success" title="Keyboard shortcuts saved" announceMode="polite" />
      {/if}

      {#if captureBindingId !== undefined}
        <Callout tone="info">
          {capturedLabel ?? "Press a shortcut"}
          <Button variant="ghost" onClick={() => onCancelCapture?.()}>Cancel capture</Button>
        </Callout>
      {/if}

      {#if visible.length === 0}
        <EmptyState
          title={query.trim() ? "No matching commands" : "No commands available"}
          message={query.trim() ? "Try a different search." : null}
        />
      {:else}
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
                      disabled={saving}
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
      {/if}

      <FormActions>
        {#if saving}
          <Button variant="ghost" disabled>Reset</Button>
        {:else}
          <ConfirmAction
            title="Reset keyboard shortcuts?"
            description="This restores the default shortcuts and clears your custom bindings."
            tone="warning"
            triggerLabel="Reset"
            confirmLabel="Reset shortcuts"
            onConfirm={() => {
              if (!saving) {
                onReset?.();
              }
            }}
          />
        {/if}
        <Button variant="secondary" disabled={!dirty || saving} onClick={() => onCancel?.()}>
          Cancel
        </Button>
        <Button
          variant="primary"
          loading={saving}
          disabled={!dirty || saving}
          onClick={() => onApply?.()}
        >
          Apply
        </Button>
      </FormActions>
    {/if}
  </Stack>
</section>
