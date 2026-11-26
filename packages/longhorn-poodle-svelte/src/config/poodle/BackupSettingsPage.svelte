<script lang="ts">
  import { formatArchiveTimestamp } from "./format.ts";
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

  import type { BackupArchiveProjection } from "@inflatable-cookie/longhorn/config";
  import ConfigSnapshotState from "./ConfigSnapshotState.svelte";
  import { configRequest, useConfigPage } from "./page-state.svelte.ts";
  import type { ConfigOperationsPageProps } from "./types.ts";

  let {
    client,
    initialSnapshot = null,
    nextRequestId = () => `config:${crypto.randomUUID()}`,
    onSnapshot,
  }: ConfigOperationsPageProps = $props();

  const initialSnapshotValue = (() => initialSnapshot)();

  const page = useConfigPage(
    initialSnapshotValue,
    () => client.snapshot(configRequest(nextRequestId)),
    (value) => {
      onSnapshot?.(value);
    },
  );

  const backup = $derived(page.snapshot?.backup ?? null);
  const canCreate = $derived(page.snapshot?.capabilities.includes("backupCreate") === true);
  const canExport = $derived(page.snapshot?.capabilities.includes("backupExport") === true);
  const canRetain = $derived(page.snapshot?.capabilities.includes("backupRetention") === true);
  const inventoryColumns: Array<TableColumn> = [
    { id: "state", label: "State", isRowHeader: true },
    { id: "path", label: "Path" },
    { id: "detail", label: "Evidence" },
  ];
  const inventoryRows = $derived<Array<TableRow>>(
    backup?.inventory.entries.map((entry, index) => ({
      id: `${index}:${entry.path ?? entry.diagnosticKind}`,
      cells: {
        state: entry.state,
        path: entry.path ?? "Operational root",
        detail: entry.detail,
      },
    })) ?? [],
  );

  async function create(pendingPolicy: "refuse" | "flush"): Promise<void> {
    await page.run("working", async () => {
      const outcome = await client.createBackup({
        ...configRequest(nextRequestId),
        pendingPolicy,
      });
      if (outcome.status === "published") {
        page.acceptSnapshot(outcome.snapshot);
        page.notice = `Backup published to ${outcome.publication.path}.`;
      } else {
        page.error = outcome.rejection.detail;
      }
    });
  }

  async function exportArchive(archive: BackupArchiveProjection): Promise<void> {
    await page.run("working", async () => {
      const outcome = await client.exportBackup({
        ...configRequest(nextRequestId),
        archiveSha256: archive.archiveSha256,
      });
      if (outcome.status === "published") {
        page.acceptSnapshot(outcome.snapshot);
        page.notice = `Backup exported to ${outcome.publication.path}.`;
      } else {
        page.error = outcome.rejection.detail;
      }
    });
  }

  async function applyRetention(): Promise<void> {
    const current = page.snapshot;
    const retention = backup?.retention;
    if (current === null || !retention) {
      return;
    }
    await page.run("working", async () => {
      const outcome = await client.applyBackupRetention({
        ...configRequest(nextRequestId),
        generation: current.generation,
        confirmationDigest: retention.confirmationDigest,
      });
      if (outcome.status === "applied") {
        page.acceptSnapshot(outcome.snapshot);
        page.notice = `${outcome.deletedPaths.length} proven archive(s) removed.`;
      } else {
        page.error = outcome.rejection.detail;
      }
    });
  }
</script>

<div aria-busy={page.busy}>
  <Stack gap="md">
    {#if page.snapshot !== null && page.error !== null}
      <Callout
        tone="danger"
        title="Backup operation failed"
        message={page.error}
        announceMode="assertive"
      />
    {/if}
    {#if page.notice}
      <Callout
        tone="success"
        title="Backup operation complete"
        message={page.notice}
        announceMode="polite"
      />
    {/if}

    {#if page.snapshot === null}
      <ConfigSnapshotState
        loadingTitle="Loading backup inventory"
        error={page.error}
        retry={page.refresh}
      />
    {:else if backup === null}
      <Callout
        tone="warning"
        title="Backup inventory unavailable"
        message="This host did not compose backup inventory for this window."
      >
        {#snippet actions()}
          <Button onClick={() => void page.refresh()} loading={page.busy}>Retry</Button>
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
                      <strong>{formatArchiveTimestamp(archive.createdAt)}</strong>
                      <p>{archive.path}</p>
                      <small>{archive.archiveSha256}</small>
                    </Stack>
                    {#if canExport}
                      <Button
                        variant="secondary"
                        disabled={page.busy}
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
        <Button variant="ghost" onClick={() => void page.refresh()} loading={page.busy}
          >Refresh</Button
        >
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
            <Button variant="primary" loading={page.busy} onClick={() => void create("refuse")}>
              Create backup
            </Button>
          {/if}
        {/if}
      </FormActions>

      {#if canRetain && backup.retention}
        <Surface asRole="region" label="Backup retention">
          <h3>Retention plan</h3>
          <p>
            {backup.retention.deletionPaths.length} proven archive(s) are eligible for deletion.
          </p>
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
