<script lang="ts">
  import { Button, Callout, InlineListSection, Spinner, Stack } from "@inflatable-cookie/poodle-svelte";
  import type { ForkBranchRecord, ForkEntryRecord } from "../generated/protocol.ts";
  import type { ForkHistorySession } from "../svelte/session.svelte.ts";
  let { session, title = "History branches" }: { session: ForkHistorySession; title?: string } = $props();
  const failure = $derived(session.status.kind === "failed" ? String(session.status.error) : null);
  function checkout(entry: ForkEntryRecord): void { const branch = session.path?.branchId ?? session.snapshot?.summary.currentBranchId; if (branch && entry.position !== "current") void session.checkout(branch, entry.entryId); }
  function branch(record: ForkBranchRecord): void { void session.selectBranchPath(record.branchId); }
</script>
<Stack gap="sm" ariaLabel={title}>
  <Stack direction="row" gap="sm">
    <Button disabled={!session.canUndo || session.navigationPending} onClick={() => void session.undo()}>Undo</Button>
    <Button disabled={!session.canRedo || session.navigationPending} onClick={() => void session.redo()}>Redo</Button>
    <Button onClick={() => void session.loadBranches()}>Branches</Button>
    <Button onClick={() => void session.selectDefaultPath()}>Current path</Button>
  </Stack>
  {#if session.status.kind === "loading"}<Spinner ariaLabel="Loading history branches" />
  {:else if failure}<Callout tone="danger" title="History unavailable" message={failure} announceMode="polite" />
  {:else}
    {#if session.branches}
      <InlineListSection title="Branches" items={session.branches.branches} count={session.branches.totalBranches} emptyMessage="No branches" framed={false}>
        {#snippet item(record)}<Button pressed={record.current} onClick={() => branch(record)}>{record.name ?? record.branchId}</Button>{/snippet}
      </InlineListSection>
    {/if}
    <InlineListSection title="Path" items={session.entries} count={session.path?.totalEntries ?? 0} emptyMessage="No history entries" framed={false}>
      {#snippet item(entry)}<Button pressed={entry.position === "current"} onClick={() => checkout(entry)}>{entry.label}</Button>{/snippet}
    </InlineListSection>
  {/if}
</Stack>
