<script lang="ts">
  import {
    Button,
    Callout,
    ConfirmAction,
    DetailItem,
    FormActions,
    Select,
    Surface,
    Table,
  } from "@inflatable-cookie/poodle-svelte";
  import type {
    SelectOption,
    TableColumn,
    TableRow,
  } from "@inflatable-cookie/poodle-svelte";
  import { onMount } from "svelte";

  import {
    CONFIG_OPERATIONS_PROTOCOL_VERSION,
    type ConfigOperationsSnapshot,
    type StorageProfileId,
    type StorageTransitionPreviewProjection,
    type StorageTransitionReceiptProjection,
  } from "../index.ts";
  import type { ConfigOperationsPageProps } from "./types.ts";

  let {
    client,
    initialSnapshot = null,
    nextRequestId = () => `config:${crypto.randomUUID()}`,
    onSnapshot,
  }: ConfigOperationsPageProps = $props();

  let snapshot = $state<ConfigOperationsSnapshot | null>(initialSnapshot);
  let selectedProfile = $state<StorageProfileId | null>(
    initialSnapshot?.storage?.layout.profile ?? null,
  );
  let preview = $state<StorageTransitionPreviewProjection | null>(null);
  let receipt = $state<StorageTransitionReceiptProjection | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);

  const storage = $derived(snapshot?.storage ?? null);
  const canTransition = $derived(
    snapshot?.capabilities.includes("storageTransition") === true,
  );
  const profileOptions = $derived<SelectOption[]>(
    storage?.availableProfiles.map((value) => ({
      value,
      label: profileLabel(value),
    })) ?? [],
  );
  const rootColumns: TableColumn[] = [
    { id: "kind", label: "Purpose", isRowHeader: true },
    { id: "path", label: "Resolved path" },
    { id: "provenance", label: "Source" },
  ];
  const rootRows = $derived<TableRow[]>(
    storage?.layout.roots.map((root) => ({
      id: root.kind,
      cells: {
        kind: root.kind,
        path: root.path,
        provenance: root.provenance,
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

  async function inspect(): Promise<void> {
    const targetProfile = selectedProfile;
    if (targetProfile === null) return;
    await run(async () => {
      receipt = null;
      const outcome = await client.inspectStorageTransition({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
        targetProfile,
        includeLogs: true,
      });
      if (outcome.status === "ready") {
        preview = outcome.preview;
        notice =
          outcome.preview.conflicts.length === 0
            ? "Transition inspected. Review the exact plan before confirming."
            : "Transition has blocking conflicts and cannot be confirmed.";
      } else {
        reject(outcome.rejection.detail);
      }
    });
  }

  async function execute(): Promise<void> {
    if (snapshot === null || preview === null) return;
    await run(async () => {
      const outcome = await client.executeStorageTransition({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
        generation: snapshot!.generation,
        confirmationDigest: preview!.confirmationDigest,
      });
      if (outcome.status === "committed") {
        receipt = outcome.receipt;
        preview = null;
        acceptSnapshot(outcome.snapshot);
        notice = `Storage transition ${outcome.receipt.transitionId} committed.`;
      } else {
        reject(outcome.rejection.detail);
      }
    });
  }

  async function recover(): Promise<void> {
    await run(async () => {
      const outcome = await client.recoverStorage({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
      });
      if (outcome.status === "recovered") {
        acceptSnapshot(outcome.snapshot);
        notice = outcome.receipt.detail;
      } else {
        reject(outcome.rejection.detail);
      }
    });
  }

  async function cleanup(): Promise<void> {
    if (receipt === null) return;
    await run(async () => {
      const outcome = await client.cleanupStorage({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
        transitionId: receipt!.transitionId,
        transitionReceiptDigest: receipt!.receiptDigest,
      });
      if (outcome.status === "applied") {
        acceptSnapshot(outcome.snapshot);
        notice = `${outcome.receipt.removedPaths.length} retained source path(s) removed.`;
        receipt = null;
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
    selectedProfile ??= value.storage?.layout.profile ?? null;
    onSnapshot?.(value);
  }

  function profileLabel(profile: StorageProfileId): string {
    switch (profile) {
      case "platform-native-v1":
        return "Platform native";
      case "unified-app-root-v1":
        return "Unified app root";
      case "shared-product-root-v1":
        return "Shared product root";
      case "portable-v1":
        return "Portable";
    }
  }
</script>

<div class="longhorn-config-page" aria-busy={busy}>
  {#if error}
    <Callout tone="danger" title="Storage operation failed" message={error} announceMode="assertive" />
  {/if}
  {#if notice}
    <Callout tone="success" title="Storage operation complete" message={notice} announceMode="polite" />
  {/if}

  {#if storage === null}
    <Callout
      tone="warning"
      title="Storage diagnostics unavailable"
      message="This host did not compose storage diagnostics for this window."
    >
      {#snippet actions()}
        <Button onClick={() => void refresh()} loading={busy}>Retry</Button>
      {/snippet}
    </Callout>
  {:else}
    {#if storage.bootstrap.state === "recoveryRequired"}
      <Callout
        tone="danger"
        title="Storage recovery required"
        message={storage.bootstrap.detail}
        announceMode="assertive"
      >
        {#snippet actions()}
          <ConfirmAction
            title="Recover storage selection?"
            description="The host will inspect the journal and locator before choosing a safe terminal state."
            tone="warning"
            triggerLabel="Recover"
            confirmLabel="Run recovery"
            onConfirm={recover}
          />
        {/snippet}
      </Callout>
    {/if}

    {#each storage.layout.warnings as warning (warning)}
      <Callout tone="warning" title="Profile consequence" message={warning} />
    {/each}

    <Surface asRole="region" label="Active storage identity">
      <div class="longhorn-config-details">
        <DetailItem label="Profile" value={profileLabel(storage.layout.profile)} />
        <DetailItem label="Application identity" value={storage.layout.canonicalApplicationId} />
        <DetailItem label="Directory leaf" value={storage.layout.effectiveLeaf} />
        <DetailItem label="Leaf source" value={storage.layout.leafProvenance} />
        <DetailItem label="Platform" value={storage.layout.platform} />
        <DetailItem label="Layout digest" value={storage.layout.layoutDigest} truncateValue={true} />
      </div>
    </Surface>

    <Table
      columns={rootColumns}
      rows={rootRows}
      caption="Resolved storage roots"
      ariaLabel="Resolved storage roots"
    />

    {#if canTransition}
      <Surface asRole="region" label="Change storage profile">
        <div class="longhorn-config-flow">
          <label for="longhorn-storage-profile">Storage profile</label>
          <Select
            id="longhorn-storage-profile"
            value={selectedProfile}
            options={profileOptions}
            native={true}
            disabled={busy}
            onValueChange={(value) => (selectedProfile = value as StorageProfileId)}
          />
          <Button
            variant="secondary"
            disabled={busy || selectedProfile === storage.layout.profile}
            onClick={() => void inspect()}
          >
            Inspect change
          </Button>
        </div>
      </Surface>
    {/if}

    {#if preview}
      <Surface asRole="region" label="Storage transition preview">
        <h3>Transition preview</h3>
        <p>{preview.domains.length} registered domain action(s).</p>
        {#if preview.unknownSourcePaths.length > 0}
          <Callout
            tone="warning"
            title="Unregistered files will be retained"
            message={preview.unknownSourcePaths.join(", ")}
          />
        {/if}
        {#each preview.conflicts as conflict (`${conflict.kind}:${conflict.path ?? ""}`)}
          <Callout tone="danger" title={conflict.kind} message={conflict.detail} />
        {/each}
        <FormActions>
          <Button variant="ghost" onClick={() => (preview = null)}>Cancel</Button>
          {#if preview.conflicts.length === 0}
            <ConfirmAction
              title="Change storage profile?"
              description="The host will recheck this exact evidence, journal the transition, and commit the locator last."
              tone="warning"
              triggerLabel="Confirm transition"
              confirmLabel="Change profile"
              onConfirm={execute}
            >
              <p>Confirmation digest: {preview.confirmationDigest}</p>
            </ConfirmAction>
          {/if}
        </FormActions>
      </Surface>
    {/if}

    {#if receipt && receipt.retainedSourcePaths.length > 0}
      <Callout
        tone="info"
        title="Old storage retained"
        message={`${receipt.retainedSourcePaths.length} source path(s) remain available for rollback.`}
      >
        {#snippet actions()}
          <ConfirmAction
            title="Remove retained old storage?"
            description="Cleanup is authorized only by this committed transition receipt and cannot be undone."
            triggerLabel="Clean up old storage"
            confirmLabel="Remove exact paths"
            onConfirm={cleanup}
          />
        {/snippet}
      </Callout>
    {/if}
  {/if}
</div>

<style>
  .longhorn-config-page,
  .longhorn-config-flow,
  .longhorn-config-details {
    display: grid;
    gap: 0.75rem;
  }

  .longhorn-config-details {
    grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
  }

  .longhorn-config-flow {
    align-items: end;
    grid-template-columns: minmax(12rem, 1fr) auto;
  }

  .longhorn-config-flow > label {
    grid-column: 1 / -1;
    font-weight: 600;
  }
</style>
