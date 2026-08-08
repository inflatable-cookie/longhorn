<script lang="ts">
  import {
    AlertDialog,
    Button,
    Callout,
    InlineListSection,
    Progress,
    Spinner,
    Stack,
    StatusIndicator,
  } from "@inflatable-cookie/poodle-svelte";
  import type { Snippet } from "svelte";

  import type { OperationEntryProjection } from "../generated/protocol.ts";
  import type { OperationSession } from "../svelte/session.svelte.ts";
  import {
    canCancelOperation,
    operationProgressView,
    operationStateLabel,
    operationStatusTone,
  } from "./projectors.ts";

  let {
    session,
    title = "Operations",
    activeTitle = "Active",
    recentTitle = "Recent",
    activeEmptyMessage = "No active operations.",
    recentEmptyMessage = "No recent operations.",
    detail,
  }: {
    session: OperationSession;
    title?: string;
    activeTitle?: string;
    recentTitle?: string;
    activeEmptyMessage?: string;
    recentEmptyMessage?: string;
    detail?: Snippet<[OperationEntryProjection]>;
  } = $props();

  let cancellationTarget = $state<OperationEntryProjection | undefined>();
  const loading = $derived(session.status.kind === "loading");
  const failure = $derived(
    session.status.kind === "failed"
      ? session.status.error instanceof Error
        ? session.status.error.message
        : String(session.status.error)
      : null,
  );

  function select(operation: OperationEntryProjection): void {
    session.select(
      session.selectedOperationId === operation.operationId
        ? undefined
        : operation.operationId,
    );
  }

  async function confirmCancellation(): Promise<void> {
    const operation = cancellationTarget;
    if (operation === undefined) return;
    await session.cancel(operation.operationId);
    cancellationTarget = undefined;
  }

  function dismiss(operation: OperationEntryProjection): void {
    void session.dismiss(operation.operationId).catch(() => undefined);
  }
</script>

{#snippet operationRow(operation: OperationEntryProjection, terminal: boolean)}
  {@const progress = operationProgressView(operation.progress.overall)}
  <Stack gap="sm">
    <Stack direction="row" gap="sm" justify="between">
      <Button
        variant={session.selectedOperationId === operation.operationId
          ? "primary"
          : "secondary"}
        pressed={session.selectedOperationId === operation.operationId}
        onClick={() => select(operation)}
      >
        {operation.label}
      </Button>
      <StatusIndicator
        status={operationStatusTone(operation.state)}
        label={operationStateLabel(operation.state)}
      />
    </Stack>

    <Progress
      indeterminate={progress.indeterminate}
      value={progress.value}
      max={progress.max}
      valueText={progress.valueText}
      ariaLabel={`${operation.label} progress`}
      size="sm"
    />

    {#if operation.progress.phase}
      <span>{operation.progress.phase.label}</span>
    {/if}

    {#if terminal}
      <Button
        variant="ghost"
        loading={session.isDismissalPending(operation.operationId)}
        disabled={session.isDismissalPending(operation.operationId)}
        ariaLabel={`Dismiss ${operation.label}`}
        onClick={() => dismiss(operation)}
      >
        Dismiss
      </Button>
    {:else if canCancelOperation(operation)}
      <Button
        variant="ghost"
        loading={session.isCancellationPending(operation.operationId)}
        disabled={session.isCancellationPending(operation.operationId)}
        ariaLabel={`Cancel ${operation.label}`}
        onClick={() => (cancellationTarget = operation)}
      >
        Cancel
      </Button>
    {/if}
  </Stack>
{/snippet}

<Stack gap="md" ariaLabel={title}>
  {#if loading}
    <Spinner ariaLabel="Loading operations" />
  {:else if failure}
    <Callout
      tone="danger"
      title="Operations unavailable"
      message={failure}
      announceMode="polite"
    />
  {:else}
    {#if session.commandRejection}
      <Callout
        tone="warning"
        title="Operation action rejected"
        message={session.commandRejection.rejection.detail}
        announceMode="polite"
      />
    {:else if session.commandFailure}
      <Callout
        tone="danger"
        title="Operation action failed"
        message={session.commandFailure.error instanceof Error
          ? session.commandFailure.error.message
          : String(session.commandFailure.error)}
        announceMode="polite"
      />
    {/if}

    <InlineListSection
      title={activeTitle}
      items={session.active}
      count={session.active.length}
      emptyMessage={activeEmptyMessage}
      framed={false}
    >
      {#snippet item(operation)}
        {@render operationRow(operation, false)}
      {/snippet}
    </InlineListSection>

    <InlineListSection
      title={recentTitle}
      items={session.recent}
      count={session.recent.length}
      emptyMessage={recentEmptyMessage}
      framed={false}
    >
      {#snippet item(operation)}
        {@render operationRow(operation, true)}
      {/snippet}
    </InlineListSection>

    {#if detail && session.selected}
      {@render detail(session.selected)}
    {/if}
  {/if}
</Stack>

<AlertDialog
  open={cancellationTarget !== undefined}
  title="Cancel operation?"
  description="Cancellation is a request. Work may finish before it stops."
  itemLabel="Operation"
  itemValue={cancellationTarget?.label ?? null}
  tone="warning"
  confirmLabel="Request cancellation"
  workingLabel="Requesting…"
  onConfirm={confirmCancellation}
  onCancel={() => (cancellationTarget = undefined)}
  onOpenChange={(open) => {
    if (!open) cancellationTarget = undefined;
  }}
/>
