<script lang="ts">
  import { Button, Callout, TextInput } from "@inflatable-cookie/poodle-svelte";

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

  <ul>
    {#each visible as record (record.id)}
      <li data-command-id={record.id}>
        <span>
          <strong>{record.label}</strong>
          {#if record.description}
            <small>{record.description}</small>
          {/if}
        </span>
        <span>
          {record.shortcuts.map(({ label }) => label).join(", ") || "Unbound"}
        </span>
        {#each record.bindings as binding (binding.id)}
          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => onCapture?.(binding.id)}
          >
            Change {binding.id}
          </Button>
        {/each}
      </li>
    {/each}
  </ul>

  <footer>
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
  </footer>
</section>

<style>
  section,
  li,
  li > span:first-child {
    display: grid;
    gap: 0.5rem;
  }

  ul {
    display: grid;
    gap: 0.75rem;
    padding: 0;
    list-style: none;
  }

  li {
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
  }

  footer {
    display: flex;
    justify-content: end;
    gap: 0.5rem;
  }
</style>
