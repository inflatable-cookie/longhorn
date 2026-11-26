<script lang="ts">
  import { formatArchiveTimestamp } from "./format.ts";
  import {
    Button,
    Callout,
    ConfirmAction,
    DetailItem,
    Field,
    FormActions,
    Grid,
    RadioGroup,
    Select,
    Stack,
    Surface,
  } from "@inflatable-cookie/poodle-svelte";
  import type { RadioGroupOption, SelectOption } from "@inflatable-cookie/poodle-svelte";

  import {
    RESTORE_AUTHENTICITY_LABELS,
    RESTORE_INTEGRITY_LABELS,
  } from "@inflatable-cookie/longhorn/config";
  import type {
    ConfigOperationRejection,
    RestoreAdapterReceiptProjection,
    RestoreDomainChoice,
    RestoreDomainInspectionProjection,
    RestoreExecutionFailureProjection,
    RestoreExecutionReceiptProjection,
    RestoreInspectionProjection,
    RestorePlanProjection,
    RestoreRecoveryReceiptProjection,
  } from "@inflatable-cookie/longhorn/config";
  import { canUseArchive, compatibilityLabel, identityLabel } from "./restore-model.ts";
  import type { RestoreChoice } from "./restore-model.ts";
  import ConfigSnapshotState from "./ConfigSnapshotState.svelte";
  import { configRequest, useConfigPage } from "./page-state.svelte.ts";
  import type { ConfigOperationsPageProps } from "./types.ts";

  let {
    client,
    initialSnapshot = null,
    nextRequestId = () => `config:${crypto.randomUUID()}`,
    onSnapshot,
  }: ConfigOperationsPageProps = $props();
  const archiveId = $props.id();

  const initialSnapshotValue = (() => initialSnapshot)();
  let selection = $state("hostPicker");
  let inspection = $state<RestoreInspectionProjection | null>(null);
  let inspectionGeneration = $state<number | null>(null);
  let choices = $state<Record<string, RestoreChoice>>({});
  let plan = $state<RestorePlanProjection | null>(null);
  let planGeneration = $state<number | null>(null);
  let failure = $state<RestoreExecutionFailureProjection | null>(null);
  let execution = $state<RestoreExecutionReceiptProjection | null>(null);
  let adapterReceipt = $state<RestoreAdapterReceiptProjection | null>(null);
  let recoveryReceipt = $state<RestoreRecoveryReceiptProjection | null>(null);

  const page = useConfigPage(
    initialSnapshotValue,
    () => client.snapshot(configRequest(nextRequestId)),
    (value) => {
      onSnapshot?.(value);
    },
  );

  const restore = $derived(page.snapshot?.restore ?? null);
  const canInspect = $derived(page.snapshot?.capabilities.includes("restoreInspection") === true);
  const canExecute = $derived(page.snapshot?.capabilities.includes("restoreExecution") === true);
  const canExecuteAdapter = $derived(
    page.snapshot?.capabilities.includes("restoreAdapterExecution") === true,
  );
  const canRecover = $derived(page.snapshot?.capabilities.includes("restoreRecovery") === true);
  const blockedByRestore = $derived(
    restore?.state === "active" || restore?.state === "recoveryRequired",
  );
  const allChoicesMade = $derived(
    inspection !== null &&
      inspection.domains.every(
        (domain) =>
          choices[domain.domainId] === "useArchive" || choices[domain.domainId] === "keepCurrent",
      ) &&
      inspection.domains.some((domain) => choices[domain.domainId] === "useArchive"),
  );
  const archiveOptions = $derived<Array<SelectOption>>([
    { value: "hostPicker", label: "Choose archive…" },
    ...(page.snapshot?.backup?.inventory.archives.map((archive) => ({
      value: archive.archiveSha256,
      label: `${formatArchiveTimestamp(archive.createdAt)} — ${archive.archiveId}`,
    })) ?? []),
  ]);
  const conflictOptions: Array<RadioGroupOption> = [
    { value: "useArchive", label: "Use archive" },
    { value: "keepCurrent", label: "Keep current" },
  ];

  async function inspect(): Promise<void> {
    await page.run("inspecting", async () => {
      clearResult();
      const outcome = await client.inspectRestore({
        ...configRequest(nextRequestId),
        selection:
          selection === "hostPicker"
            ? { source: "hostPicker" }
            : { source: "inventory", archiveSha256: selection },
      });
      if (outcome.status === "ready") {
        inspection = outcome.inspection;
        inspectionGeneration = outcome.generation;
        choices = Object.fromEntries(
          outcome.inspection.domains.map((domain) => [domain.domainId, ""]),
        );
      } else if (outcome.status === "locked") {
        page.error = `Archive locked: ${outcome.detail}`;
      } else {
        reject(outcome.rejection);
      }
    });
  }

  function isDecidedChoice(
    choice: RestoreChoice | undefined,
  ): choice is Exclude<RestoreChoice, ""> {
    return choice === "useArchive" || choice === "keepCurrent";
  }

  async function createPlan(): Promise<void> {
    const current = inspection;
    const generation = inspectionGeneration;
    if (current === null || generation === null || !allChoicesMade) {
      return;
    }
    await page.run("planning", async () => {
      const domainChoices = current.domains.map((domain): RestoreDomainChoice => {
        const choice = choices[domain.domainId];
        if (!isDecidedChoice(choice)) {
          throw new Error(`no restore choice recorded for ${domain.domainId}`);
        }
        return { domainId: domain.domainId, choice };
      });
      const outcome = await client.planRestore({
        ...configRequest(nextRequestId),
        generation,
        archiveSha256: current.archiveSha256,
        choices: domainChoices,
      });
      if (outcome.status === "ready") {
        plan = outcome.plan;
        planGeneration = outcome.generation;
      } else {
        reject(outcome.rejection);
      }
    });
  }

  async function execute(): Promise<void> {
    const current = plan;
    const generation = planGeneration;
    if (current === null || generation === null) {
      return;
    }
    await page.run("publishing", async () => {
      const outcome = await client.executeRestore({
        ...configRequest(nextRequestId),
        generation,
        confirmationDigest: current.confirmationDigest,
      });
      if (outcome.status === "succeeded") {
        execution = outcome.receipt;
        page.acceptSnapshot(outcome.snapshot);
        page.notice = "Restore published and verified.";
      } else if (outcome.status === "rolledBack") {
        failure = outcome.failure;
        page.acceptSnapshot(outcome.snapshot);
        page.notice = "Restore failed and the host verified rollback.";
      } else if (outcome.status === "recoveryRequired") {
        failure = outcome.failure;
        page.acceptSnapshot(outcome.snapshot);
        page.error = "Restore could not establish rollback. Recovery is required.";
      } else {
        reject(outcome.rejection);
      }
    });
  }

  async function executeAdapter(
    domainId: string,
    confirmationDigest: string,
    requirement: "failureAtomic" | "allowSeparate",
  ): Promise<void> {
    const current = inspection;
    const generation = inspectionGeneration;
    if (current === null || generation === null) {
      return;
    }
    await page.run("publishing", async () => {
      const outcome = await client.executeAdapterRestore({
        ...configRequest(nextRequestId),
        generation,
        archiveSha256: current.archiveSha256,
        domainId,
        confirmationDigest,
        requirement,
      });
      if (outcome.status === "completed") {
        adapterReceipt = outcome.receipt;
        page.acceptSnapshot(outcome.snapshot);
        page.notice = `Adapter restore finished with ${outcome.receipt.outcome}.`;
      } else {
        reject(outcome.rejection);
      }
    });
  }

  function executeInspectedAdapter(domain: RestoreDomainInspectionProjection): Promise<void> {
    if (domain.compatibility.status !== "customAdapterReady") {
      return Promise.resolve();
    }
    return executeAdapter(
      domain.domainId,
      domain.compatibility.confirmationDigest,
      domain.compatibility.participation.kind === "failureAtomic"
        ? "failureAtomic"
        : "allowSeparate",
    );
  }

  async function recover(): Promise<void> {
    await page.run("recovering", async () => {
      const outcome = await client.recoverRestore({
        ...configRequest(nextRequestId),
      });
      if (outcome.status === "recovered") {
        recoveryReceipt = outcome.receipt;
        page.acceptSnapshot(outcome.snapshot);
        page.notice = `Recovery finished with ${outcome.receipt.outcome}.`;
      } else if (outcome.status === "recoveryRequired") {
        failure = outcome.failure;
        page.acceptSnapshot(outcome.snapshot);
        page.error = "Recovery remains required.";
      } else {
        reject(outcome.rejection);
      }
    });
  }

  function choose(domainId: string, choice: string): void {
    choices = { ...choices, [domainId]: choice as RestoreChoice };
    plan = null;
    planGeneration = null;
  }

  function reject(rejection: ConfigOperationRejection): void {
    page.error = rejection.detail;
    if (rejection.snapshot) {
      page.acceptSnapshot(rejection.snapshot);
    }
    if (rejection.code === "restorePlanStale" || rejection.code === "authorityChanged") {
      plan = null;
      planGeneration = null;
    }
  }

  function clearResult(): void {
    inspection = null;
    inspectionGeneration = null;
    choices = {};
    plan = null;
    planGeneration = null;
    failure = null;
    execution = null;
    adapterReceipt = null;
  }
</script>

<div aria-busy={page.activity !== "idle"}>
  <Stack gap="md">
    {#if page.snapshot !== null && page.error !== null}
      <Callout
        tone="danger"
        title="Restore operation failed"
        message={page.error}
        announceMode="assertive"
      />
    {/if}
    {#if page.notice}
      <Callout
        tone="success"
        title="Restore operation complete"
        message={page.notice}
        announceMode="polite"
      />
    {/if}

    {#if page.snapshot === null}
      <ConfigSnapshotState
        loadingTitle="Loading restore inspection"
        error={page.error}
        retry={page.refresh}
      />
    {:else if restore === null}
      <Callout
        tone="warning"
        title="Restore unavailable"
        message="This host did not compose restore inspection for this window."
      />
    {:else if blockedByRestore}
      <Callout
        tone="danger"
        title={restore.state === "active" ? "Restore publication in progress" : "Recovery required"}
        message="Configuration reads and mutations remain blocked until the host establishes a terminal restore state."
        announceMode="assertive"
      >
        {#snippet actions()}
          {#if canRecover}
            <ConfirmAction
              title="Run restore recovery?"
              description="The host will inspect the durable restore journal and establish rollback or completed publication."
              triggerLabel="Run recovery"
              confirmLabel="Confirm recovery"
              onConfirm={recover}
            />
          {/if}
        {/snippet}
      </Callout>
    {:else}
      <Surface asRole="region" label="Select restore archive">
        <Field id={archiveId} label="Backup archive">
          <Grid columns="minmax(12rem, 1fr) auto" gap="sm">
            <Stack>
              <Select
                id={archiveId}
                value={selection}
                options={archiveOptions}
                native={true}
                disabled={page.activity !== "idle" || !canInspect}
                onValueChange={(value) => (selection = value)}
              />
            </Stack>
            <Stack direction="row" align="end">
              <Button
                variant="secondary"
                loading={page.activity === "inspecting"}
                disabled={page.activity !== "idle" || !canInspect}
                onClick={() => void inspect()}
              >
                Inspect archive
              </Button>
            </Stack>
          </Grid>
        </Field>
        <p>Paths, unlock material, and archive bytes remain inside host authority.</p>
      </Surface>

      {#if inspection}
        <Surface asRole="region" label="Verified archive inspection">
          <h3>Verified archive</h3>
          <Grid columns="repeat(auto-fit, minmax(14rem, 1fr))" gap="md">
            <DetailItem label="Archive" value={inspection.archiveId} />
            <DetailItem label="Created" value={formatArchiveTimestamp(inspection.createdAt)} />
            <DetailItem label="Integrity" value={RESTORE_INTEGRITY_LABELS[inspection.integrity]} />
            <DetailItem
              label="Authenticity"
              value={RESTORE_AUTHENTICITY_LABELS[inspection.authenticity]}
            />
            <DetailItem
              label="Application identity"
              value={identityLabel(inspection.identity.application)}
            />
            <DetailItem
              label="Producer identity"
              value={identityLabel(inspection.identity.producer)}
            />
            <DetailItem
              label="Archive digest"
              value={inspection.archiveSha256}
              truncateValue={true}
            />
          </Grid>
        </Surface>

        <section aria-label="Restore conflict choices">
          <h3>Domain choices</h3>
          {#each inspection.domains as domain (domain.domainId)}
            <Surface asRole="group" label={domain.domainId}>
              <Grid columns="minmax(0, 1fr) minmax(12rem, auto)" gap="md">
                <Stack gap="sm">
                  <strong>{domain.domainId}</strong>
                  <p>{compatibilityLabel(domain.compatibility)}</p>
                  <small
                    >{domain.storageClass} · {domain.consistencyGroup} · {domain.sourceState}</small
                  >
                </Stack>
                <Stack>
                  <RadioGroup
                    ariaLabel={`Restore choice for ${domain.domainId}`}
                    value={choices[domain.domainId]}
                    options={conflictOptions.map((option) => ({
                      ...option,
                      disabled: option.value === "useArchive" && !canUseArchive(domain),
                    }))}
                    disabled={page.activity !== "idle"}
                    onValueChange={(value) => choose(domain.domainId, value)}
                  />
                </Stack>
              </Grid>
              {#if domain.compatibility.status === "customAdapterReady" && canExecuteAdapter}
                <ConfirmAction
                  title={`Restore ${domain.domainId} with ${domain.compatibility.adapter}?`}
                  description="This is a separate adapter-owned operation with its own terminal receipt."
                  triggerLabel="Restore with adapter…"
                  confirmLabel="Run adapter restore"
                  onConfirm={() => executeInspectedAdapter(domain)}
                />
              {/if}
            </Surface>
          {/each}
        </section>

        {#if inspection.consistencyGroups.length > 0}
          <Surface asRole="region" label="Archive consistency groups">
            <h3>Consistency groups</h3>
            <Stack gap="sm" asRole="list">
              {#each inspection.consistencyGroups as group (group.id)}
                <span role="listitem"
                  ><strong>{group.id}</strong>: {group.mode} via {group.authority}</span
                >
              {/each}
            </Stack>
          </Surface>
        {/if}

        {#if inspection.exclusions.length > 0}
          <Callout
            tone="info"
            title="Archive exclusions"
            message={inspection.exclusions
              .map((excluded) => `${excluded.domainId}: ${excluded.reason}`)
              .join(", ")}
          />
        {/if}

        <FormActions>
          <Button
            variant="secondary"
            loading={page.activity === "planning"}
            disabled={page.activity !== "idle" || !allChoicesMade}
            onClick={() => void createPlan()}
          >
            Review exact plan
          </Button>
        </FormActions>
      {/if}

      {#if plan}
        <Surface asRole="region" label="Exact restore plan">
          <h3>Exact restore plan</h3>
          <p>
            {plan.receipt.selected} selected; {plan.receipt.skipped} preserved;
            {plan.receipt.migrations} migration(s).
          </p>
          <Stack gap="sm" asRole="list">
            {#each plan.entries as entry (entry.domainId)}
              <span role="listitem">
                <strong>{entry.domainId}</strong>: {entry.choice}
                {#if entry.action}
                  → {entry.action}{/if}
                {#if entry.current?.state === "present"}
                  ({entry.current.byteLength} bytes, {entry.current.sha256})
                {/if}
              </span>
            {/each}
          </Stack>
          <p>{plan.confirmationDigest}</p>
          {#if canExecute}
            <ConfirmAction
              title="Publish this exact restore plan?"
              description="The host will recheck current evidence, stage privately, create a safety backup, then publish under a durable journal."
              triggerLabel="Restore selected domains…"
              confirmLabel="Publish restore"
              onConfirm={execute}
            />
          {/if}
        </Surface>
      {/if}
    {/if}

    {#if page.activity === "publishing"}
      <Callout
        tone="warning"
        title="Host-owned publication in progress"
        message="Closing this view does not cancel staging, safety backup, publication, rollback, or recovery."
        announceMode="assertive"
      />
    {:else if page.activity === "recovering"}
      <Callout
        tone="warning"
        title="Host-owned recovery in progress"
        message="Closing this view does not cancel recovery."
        announceMode="assertive"
      />
    {/if}

    {#if execution}
      <Callout
        tone="success"
        title="Verified restore receipt"
        message={`${execution.restoredDomainIds.length} domain(s) restored; safety backup ${execution.safetyBackup.archiveSha256}.`}
      />
    {/if}
    {#if failure}
      <Callout
        tone={failure.terminal === "recoveryRequired" ? "danger" : "warning"}
        title={`Restore terminal: ${failure.terminal}`}
        message={`${failure.stage}: ${failure.detail}`}
      />
    {/if}
    {#if adapterReceipt}
      <Callout
        tone={adapterReceipt.outcome === "recoveryRequired" ? "danger" : "info"}
        title="Adapter restore receipt"
        message={`${adapterReceipt.domainId}: ${adapterReceipt.outcome}`}
      />
    {/if}
    {#if recoveryReceipt}
      <Callout
        tone="success"
        title="Recovery receipt"
        message={`${recoveryReceipt.outcome}; ${recoveryReceipt.domainIds.length} domain(s) considered.`}
      />
    {/if}
  </Stack>
</div>
