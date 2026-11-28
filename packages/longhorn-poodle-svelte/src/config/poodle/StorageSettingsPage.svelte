<script lang="ts">
  import {
    Button,
    Callout,
    ConfirmAction,
    DetailItem,
    Field,
    FormActions,
    Grid,
    Select,
    Stack,
    Surface,
    Table,
  } from "@inflatable-cookie/poodle-svelte";
  import type { SelectOption, TableColumn, TableRow } from "@inflatable-cookie/poodle-svelte";

  import type {
    StorageLeafProvenanceProjection,
    StorageProfileId,
    StorageTransitionPreviewProjection,
    StorageTransitionReceiptProjection,
  } from "@inflatable-cookie/longhorn/config";
  import ConfigSnapshotState from "./ConfigSnapshotState.svelte";
  import { configRequest, useConfigPage } from "./page-state.svelte.ts";
  import type { ConfigOperationsPageProps } from "./types.ts";

  let {
    client,
    initialSnapshot = null,
    nextRequestId = () => `config:${crypto.randomUUID()}`,
    onSnapshot,
  }: ConfigOperationsPageProps = $props();
  const profileId = $props.id();

  const initialSnapshotValue = (() => initialSnapshot)();
  const initialProfileValue = (() => initialSnapshot?.storage?.layout.profile ?? null)();
  let selectedProfile = $state<StorageProfileId | null>(initialProfileValue);
  let preview = $state<StorageTransitionPreviewProjection | null>(null);
  let receipt = $state<StorageTransitionReceiptProjection | null>(null);

  const page = useConfigPage(
    initialSnapshotValue,
    () => client.snapshot(configRequest(nextRequestId)),
    (value) => {
      selectedProfile ??= value.storage?.layout.profile ?? null;
      onSnapshot?.(value);
    },
  );

  const storage = $derived(page.snapshot?.storage ?? null);
  const leafSourceLabels: Record<StorageLeafProvenanceProjection, string> = {
    canonicalApplicationId: "Application identity",
    stableStorageName: "Configured storage name",
  };
  const platformLabels = new Map([
    ["macos", "macOS"],
    ["linux", "Linux"],
    ["windows", "Windows"],
  ]);
  const conflictTitles = new Map([
    ["overlappingRoots", "Storage roots overlap"],
    ["targetOccupied", "Destination contains different data"],
    ["unknownTargetFile", "Destination contains an unregistered file"],
  ]);
  const canTransition = $derived(
    page.snapshot?.capabilities.includes("storageTransition") === true,
  );
  const profileOptions = $derived<Array<SelectOption>>(
    storage?.availableProfiles.map((value) => ({
      value,
      label: profileLabel(value),
    })) ?? [],
  );
  const rootColumns: Array<TableColumn> = [
    { id: "kind", label: "Purpose", isRowHeader: true },
    { id: "path", label: "Resolved path" },
    { id: "provenance", label: "Source" },
  ];
  const rootRows = $derived<Array<TableRow>>(
    storage?.layout.roots.map((root) => ({
      id: root.kind,
      cells: {
        kind: root.kind,
        path: root.path,
        provenance: root.provenance,
      },
    })) ?? [],
  );

  async function inspect(): Promise<void> {
    const targetProfile = selectedProfile;
    if (targetProfile === null) {
      return;
    }
    await page.run("working", async () => {
      receipt = null;
      const outcome = await client.inspectStorageTransition({
        ...configRequest(nextRequestId),
        targetProfile,
        includeLogs: true,
      });
      if (outcome.status === "ready") {
        preview = outcome.preview;
        page.notice =
          outcome.preview.conflicts.length === 0
            ? "Transition inspected. Review the exact plan before confirming."
            : "Transition has blocking conflicts and cannot be confirmed.";
      } else {
        page.error = outcome.rejection.detail;
      }
    });
  }

  async function execute(): Promise<void> {
    const inspected = preview;
    const current = page.snapshot;
    if (current === null || inspected === null) {
      return;
    }
    await page.run("working", async () => {
      const outcome = await client.executeStorageTransition({
        ...configRequest(nextRequestId),
        generation: current.generation,
        confirmationDigest: inspected.confirmationDigest,
      });
      if (outcome.status === "committed") {
        receipt = outcome.receipt;
        preview = null;
        page.acceptSnapshot(outcome.snapshot);
        page.notice = `Storage transition ${outcome.receipt.transitionId} committed.`;
      } else {
        page.error = outcome.rejection.detail;
      }
    });
  }

  async function recover(): Promise<void> {
    await page.run("working", async () => {
      const outcome = await client.recoverStorage({
        ...configRequest(nextRequestId),
      });
      if (outcome.status === "recovered") {
        page.acceptSnapshot(outcome.snapshot);
        page.notice = outcome.receipt.detail;
      } else {
        page.error = outcome.rejection.detail;
      }
    });
  }

  async function cleanup(): Promise<void> {
    const committed = receipt;
    if (committed === null) {
      return;
    }
    await page.run("working", async () => {
      const outcome = await client.cleanupStorage({
        ...configRequest(nextRequestId),
        transitionId: committed.transitionId,
        transitionReceiptDigest: committed.receiptDigest,
      });
      if (outcome.status === "applied") {
        page.acceptSnapshot(outcome.snapshot);
        page.notice = `${outcome.receipt.removedPaths.length} retained source path(s) removed.`;
        receipt = null;
      } else {
        page.error = outcome.rejection.detail;
      }
    });
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

<div aria-busy={page.busy}>
  <Stack gap="md">
    {#if page.snapshot !== null && page.error !== null}
      <Callout
        tone="danger"
        title="Storage operation failed"
        message={page.error}
        announceMode="assertive"
      />
    {/if}
    {#if page.notice}
      <Callout
        tone="success"
        title="Storage operation complete"
        message={page.notice}
        announceMode="polite"
      />
    {/if}

    {#if page.snapshot === null}
      <ConfigSnapshotState
        loadingTitle="Loading storage diagnostics"
        error={page.error}
        retry={page.refresh}
      />
    {:else if storage === null}
      <Callout
        tone="warning"
        title="Storage diagnostics unavailable"
        message="This host did not compose storage diagnostics for this window."
      >
        {#snippet actions()}
          <Button onClick={() => void page.refresh()} loading={page.busy}>Retry</Button>
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
        <Grid columns="repeat(auto-fit, minmax(14rem, 1fr))" gap="md">
          <DetailItem label="Profile" value={profileLabel(storage.layout.profile)} />
          <DetailItem label="Application identity" value={storage.layout.canonicalApplicationId} />
          <DetailItem label="Directory leaf" value={storage.layout.effectiveLeaf} />
          <DetailItem label="Leaf source" value={leafSourceLabels[storage.layout.leafProvenance]} />
          <DetailItem
            label="Platform"
            value={platformLabels.get(storage.layout.platform) ?? "Unknown platform"}
          />
          <DetailItem
            label="Layout digest"
            value={storage.layout.layoutDigest}
            truncateValue={true}
          />
        </Grid>
      </Surface>

      <Table
        columns={rootColumns}
        rows={rootRows}
        caption="Resolved storage roots"
        ariaLabel="Resolved storage roots"
      />

      {#if canTransition}
        <Surface asRole="region" label="Change storage profile">
          <Field id={profileId} label="Storage profile">
            <Grid columns="minmax(12rem, 1fr) auto" gap="sm">
              <Stack>
                <Select
                  id={profileId}
                  value={selectedProfile}
                  options={profileOptions}
                  native={true}
                  disabled={page.busy}
                  onValueChange={(value) => (selectedProfile = value as StorageProfileId)}
                />
              </Stack>
              <Stack direction="row" align="end">
                <Button
                  variant="secondary"
                  disabled={page.busy || selectedProfile === storage.layout.profile}
                  onClick={() => void inspect()}
                >
                  Inspect change
                </Button>
              </Stack>
            </Grid>
          </Field>
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
            <Callout
              tone="danger"
              title={conflictTitles.get(conflict.kind) ?? "Storage conflict"}
              message={conflict.detail}
            />
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
  </Stack>
</div>
