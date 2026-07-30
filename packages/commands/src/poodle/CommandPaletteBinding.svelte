<script lang="ts">
  import { CommandPalette } from "@poodle/svelte";

  import type { CommandSession } from "../svelte/session.svelte.ts";
  import {
    toPoodleCommandItems,
    toPoodleDiscoveryState,
  } from "./projectors.ts";

  interface Props {
    session: CommandSession;
    title?: string;
    description?: string | null;
    invocationHint?: string | null;
    categoryLabel?: (categoryPath: readonly string[]) => string | null;
  }

  let {
    session,
    title = "Command palette",
    description = null,
    invocationHint = null,
    categoryLabel = undefined,
  }: Props = $props();

  let records = $derived(session.paletteRecords);
  let items = $derived(toPoodleCommandItems(records, categoryLabel));
  let state = $derived(
    toPoodleDiscoveryState(session.status, session.query, items.length),
  );
</script>

<CommandPalette
  open={session.open}
  query={session.query}
  {items}
  {state}
  {title}
  {description}
  {invocationHint}
  onOpenChange={(open) => session.setOpen(open)}
  onQueryChange={(query) => void session.setQuery(query)}
  onCommandSelect={(commandId) => {
    void session.select(commandId);
  }}
/>
