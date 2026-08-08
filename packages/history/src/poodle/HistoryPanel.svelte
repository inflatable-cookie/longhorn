<script lang="ts">
  import {
    Button,
    Callout,
    InlineListSection,
    Pagination,
    Spinner,
    Stack,
    TextInput,
  } from "@inflatable-cookie/poodle-svelte";

  import type { HistoryEntryRecord } from "../generated/protocol.ts";
  import type { HistorySession } from "../svelte/session.svelte.ts";

  let {
    session,
    title = "History",
    emptyMessage = "No retained history entries.",
  }: {
    session: HistorySession;
    title?: string;
    emptyMessage?: string;
  } = $props();

  const loading = $derived(session.status.kind === "loading");
  const failure = $derived(
    session.status.kind === "failed"
      ? session.status.error instanceof Error
        ? session.status.error.message
        : String(session.status.error)
      : null,
  );

  function checkout(entry: HistoryEntryRecord): void {
    if (entry.position !== "current") void session.checkout(entry.entryId);
  }
</script>

<Stack gap="sm" ariaLabel={title}>
  <Stack direction="row" gap="sm">
    <Button
      disabled={!session.canUndo || session.navigationPending}
      loading={session.navigationPending}
      onClick={() => void session.undo()}
    >
      {session.snapshot?.summary.nextUndoLabel
        ? `Undo ${session.snapshot.summary.nextUndoLabel}`
        : "Undo"}
    </Button>
    <Button
      disabled={!session.canRedo || session.navigationPending}
      onClick={() => void session.redo()}
    >
      {session.snapshot?.summary.nextRedoLabel
        ? `Redo ${session.snapshot.summary.nextRedoLabel}`
        : "Redo"}
    </Button>
  </Stack>

  <TextInput
    type="search"
    value={session.filter}
    placeholder="Filter history"
    ariaLabel="Filter history"
    onValueChange={(value) => session.setFilter(value)}
  />

  {#if loading}
    <Spinner ariaLabel="Loading history" />
  {:else if failure}
    <Callout
      tone="danger"
      title="History unavailable"
      message={failure}
      announceMode="polite"
    />
  {:else}
    {#if session.rejection}
      <Callout
        tone="warning"
        title="History action rejected"
        message={session.rejection.detail}
        announceMode="polite"
      />
    {/if}

    <InlineListSection
      {title}
      items={session.entries}
      count={session.totalEntries}
      {emptyMessage}
      framed={false}
    >
      {#snippet item(entry)}
        <Button
          variant={entry.position === "current" ? "primary" : "secondary"}
          pressed={entry.position === "current"}
          disabled={session.navigationPending}
          onClick={() => checkout(entry)}
        >
          {entry.label}
        </Button>
      {/snippet}
    </InlineListSection>

    <Pagination
      page={session.page}
      limit={session.pageSize}
      total={session.totalEntries}
      loading={session.navigationPending}
      showLimitSelector={true}
      limitOptions={[25, 50, 100]}
      ariaLabel="History pages"
      onPageChange={(page) => void session.setPage(page)}
      onLimitChange={(limit) => void session.setPageSize(limit)}
    />
  {/if}
</Stack>
