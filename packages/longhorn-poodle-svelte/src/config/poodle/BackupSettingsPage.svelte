<script lang="ts">
  import {
    Button,
    Callout,
    ConfirmAction,
    DetailItem,
    FormActions,
    Grid,
    Stack,
    Surface,
    Table,
  } from "@inflatable-cookie/poodle-svelte";
  import type { TableColumn, TableRow } from "@inflatable-cookie/poodle-svelte";
  import { onMount } from "svelte";

  import {
    CONFIG_OPERATIONS_PROTOCOL_VERSION,
    type BackupArchiveProjection,
    type ConfigOperationsSnapshot,
  } from "@inflatable-cookie/longhorn/config";
  import type { ConfigOperationsPageProps } from "./types.ts";

  let {
    client,
    initialSnapshot = null,
    nextRequestId = () => `config:${crypto.randomUUID()}`,
    onSnapshot,
  }: ConfigOperationsPageProps = $props();

  const initialSnapshotValue = (() => initialSnapshot)();
  let snapshot = $state<ConfigOperationsSnapshot | null>(initialSnapshotValue);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);

  const backup = $derived(snapshot?.backup ?? null);
  const canCreate = $derived(snapshot?.capabilities.includes("backupCreate") === true);
  const canExport = $derived(snapshot?.capabilities.includes("backupExport") === true);
  const canRetain = $derived(snapshot?.capabilities.includes("backupRetention") === true);
  const inventoryColumns: TableColumn[] = [
    { id: "state", label: "State", isRowHeader: true },
    { id: "path", label: "Path" },
    { id: "detail", label: "Evidence" },
  ];
  const inventoryRows = $derived<TableRow[]>(
    backup?.inventory.entries.map((entry, index) => ({
      id: `${index}:${entry.path ?? entry.diagnosticKind}`,
      cells: {
        state: entry.state,
        path: entry.path ?? "Operational root",
        detail: entry.detail,
      },
    })) ?? [],
  );

  onMount(() => {
    if (snapshot === null) void refresh();
  });

  async function refresh(): Promise<void> {
    await run(async () => {
      acceptSnapshot(
        await client.snapshot({
          protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
          requestId: nextRequestId(),
        }),
      );
    });
  }

  async function create(pendingPolicy: "refuse" | "flush"): Promise<void> {
    await run(async () => {
      const outcome = await client.createBackup({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
        pendingPolicy,
      });
      if (outcome.status === "published") {
        acceptSnapshot(outcome.snapshot);
        notice = `Backup published to ${outcome.publication.path}.`;
      } else {
        reject(outcome.rejection.detail);
      }
    });
  }

  async function exportArchive(archive: BackupArchiveProjection): Promise<void> {
    await run(async () => {
      const outcome = await client.exportBackup({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
        archiveSha256: archive.archiveSha256,
      });
      if (outcome.status === "published") {
        acceptSnapshot(outcome.snapshot);
        notice = `Backup exported to ${outcome.publication.path}.`;
      } else {
        reject(outcome.rejection.detail);
      }
    });
  }

  async function applyRetention(): Promise<void> {
    if (snapshot === null || backup?.retention === null) return;
    await run(async () => {
      const outcome = await client.applyBackupRetention({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
        generation: snapshot!.generation,
        confirmationDigest: backup!.retention!.confirmationDigest,
      });
      if (outcome.status === "applied") {
        acceptSnapshot(outcome.snapshot);
        notice = `${outcome.deletedPaths.length} proven archive(s) removed.`;
      } else {
        reject(outcome.rejection.detail);
      }
    });
  }

  async function run(action: () => Promise<void>): Promise<void> {
    busy = true;
    error = null;
    notice = null;
    try {
      await action();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      busy = false;
    }
  }

  function reject(detail: string): void {
    error = detail;
  }

  function acceptSnapshot(value: ConfigOperationsSnapshot): void {
    snapshot = value;
    onSnapshot?.(value);
  }
</script>

<div aria-busy={busy}>
  <Stack gap="md">
    {#if error}
      <Callout tone="danger" title="Backup operation failed" message={error} announceMode="assertive" />
    {/if}
    {#if notice}
      <Callout tone="success" title="Backup operation complete" message={notice} announceMode="polite" />
    {/if}

    <!-- Three different situations used to render this one warning: the first
         read still in flight, a read that failed, and a host that genuinely
         composed no backup inventory. Only the third is what the message claims, and a
         Retry beside the other two is either pointless or premature. The load
         is now its own state, so what a reader sees is what happened. -->
    {#if backup === null && snapshot === null && busy}
      <Callout tone="pending" title="Loading backup inventory" announceMode="polite" />
    {:else if snapshot === null}
      <Callout
        tone="warning"
        title="Settings could not be read"
        message="No configuration snapshot arrived. The read did not fail, so the host answered with nothing."
      >
        {#snippet actions()}
          <Button onClick={() => void refresh()}>Retry</Button>
        {/snippet}
      </Callout>
    {:else if backup === null}
      <Callout
        tone="warning"
        title="Backup inventory unavailable"
        message="This host did not compose backup inventory for this window."
      >
        {#snippet actions()}
          <Button onClick={() => void refresh()} loading={busy}>Retry</Button>
        {/snippet}
      </Callout>
    {:else}
      {#if backup.pending.state === "pending"}
        <Callout
          tone="warning"
          title="Unpublished configuration changes"
          message={`${backup.pending.domainCount} domain(s) must be refused or explicitly flushed before capture.`}
        />
      {/if}

      {#if backup.encryption.state === "interactionRequired"}
        <Callout
          tone="warning"
          title="Encryption interaction required"
          message="Encryption identity stays in the host provider. Complete host interaction before retrying."
        />
      {:else if backup.encryption.state === "failed"}
        <Callout tone="danger" title="Encryption unavailable" message={backup.encryption.detail} />
      {/if}

      <Surface asRole="region" label="Backup status">
        <Grid columns="repeat(auto-fit, minmax(14rem, 1fr))" gap="md">
          <DetailItem label="Operational root" value={backup.inventory.root} />
          <DetailItem label="Inventory complete" value={backup.inventory.complete ? "Yes" : "No"} />
          <DetailItem label="Valid archives" value={backup.inventory.archives.length} />
          <DetailItem label="Encryption" value={backup.encryption.state} />
        </Grid>
      </Surface>

      <section aria-label="Operational backups">
        <h3>Operational backups</h3>
        {#if backup.inventory.archives.length === 0}
          <p>No valid same-app backups found.</p>
        {:else}
          <Stack gap="md" asRole="list">
            {#each backup.inventory.archives as archive (archive.archiveSha256)}
              <div role="listitem">
                <Surface asRole="group" label={`Backup ${archive.archiveId}`}>
                  <Stack direction="row" justify="between" align="center" gap="md">
                    <Stack gap="sm">
                      <strong>{archive.createdAt}</strong>
                      <p>{archive.path}</p>
                      <small>{archive.archiveSha256}</small>
                    </Stack>
                    {#if canExport}
                      <Button
                        variant="secondary"
                        disabled={busy}
                        onClick={() => void exportArchive(archive)}
                      >
                        Export…
                      </Button>
                    {/if}
                  </Stack>
                </Surface>
              </div>
            {/each}
          </Stack>
        {/if}
      </section>

      {#if backup.inventory.entries.length > 0}
        <Callout
          tone="info"
          title="Preserved inventory"
          message="Locked, corrupt, foreign, unknown, unreadable, and unmanaged entries are excluded from automatic retention."
        />
        <Table
          columns={inventoryColumns}
          rows={inventoryRows}
          caption="Preserved backup inventory"
          ariaLabel="Preserved backup inventory"
        />
      {/if}

      <FormActions>
        <Button variant="ghost" onClick={() => void refresh()} loading={busy}>Refresh</Button>
        {#if canCreate}
          {#if backup.pending.state === "pending"}
            <ConfirmAction
              title="Flush and create backup?"
              description="The host will durably publish pending configuration before capture."
              tone="warning"
              triggerLabel="Flush and create"
              confirmLabel="Flush and create backup"
              onConfirm={() => create("flush")}
            />
          {:else}
            <Button variant="primary" loading={busy} onClick={() => void create("refuse")}>
              Create backup
            </Button>
          {/if}
        {/if}
      </FormActions>

      {#if canRetain && backup.retention}
        <Surface asRole="region" label="Backup retention">
          <h3>Retention plan</h3>
          <p>{backup.retention.deletionPaths.length} proven archive(s) are eligible for deletion.</p>
          <ConfirmAction
            title="Apply backup retention?"
            description="The host will recheck every exact path and archive digest. Protected or uninspectable entries remain untouched."
            triggerLabel="Apply retention"
            confirmLabel="Delete proven archives"
            onConfirm={applyRetention}
          >
            <Stack gap="sm" asRole="list">
              {#each backup.retention.deletionPaths as path (path)}
                <span role="listitem">{path}</span>
              {/each}
            </Stack>
          </ConfirmAction>
        </Surface>
      {/if}
    {/if}
  </Stack>
</div>
