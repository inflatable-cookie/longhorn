<script lang="ts">
  import {
    Button,
    Callout,
    ConfirmAction,
    DetailItem,
    FormActions,
    RadioGroup,
    Select,
    Surface,
  } from "@inflatable-cookie/poodle-svelte";
  import type { RadioGroupOption, SelectOption } from "@inflatable-cookie/poodle-svelte";
  import { onMount } from "svelte";

  import {
    CONFIG_OPERATIONS_PROTOCOL_VERSION,
    RESTORE_AUTHENTICITY_LABELS,
    RESTORE_INTEGRITY_LABELS,
    type ConfigOperationRejection,
    type ConfigOperationsSnapshot,
    type RestoreAdapterReceiptProjection,
    type RestoreDomainChoice,
    type RestoreDomainInspectionProjection,
    type RestoreExecutionFailureProjection,
    type RestoreExecutionReceiptProjection,
    type RestoreInspectionProjection,
    type RestorePlanProjection,
    type RestoreRecoveryReceiptProjection,
  } from "@inflatable-cookie/longhorn/config";
  import {
    canUseArchive,
    compatibilityLabel,
    identityLabel,
    type RestoreChoice,
  } from "./restore-model.ts";
  import type { ConfigOperationsPageProps } from "./types.ts";

  type Activity = "idle" | "inspecting" | "planning" | "publishing" | "recovering";

  let {
    client,
    initialSnapshot = null,
    nextRequestId = () => `config:${crypto.randomUUID()}`,
    onSnapshot,
  }: ConfigOperationsPageProps = $props();

  let snapshot = $state<ConfigOperationsSnapshot | null>(initialSnapshot);
  let selection = $state("hostPicker");
  let inspection = $state<RestoreInspectionProjection | null>(null);
  let inspectionGeneration = $state<number | null>(null);
  let choices = $state<Record<string, RestoreChoice>>({});
  let plan = $state<RestorePlanProjection | null>(null);
  let planGeneration = $state<number | null>(null);
  let activity = $state<Activity>("idle");
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let failure = $state<RestoreExecutionFailureProjection | null>(null);
  let execution = $state<RestoreExecutionReceiptProjection | null>(null);
  let adapterReceipt = $state<RestoreAdapterReceiptProjection | null>(null);
  let recoveryReceipt = $state<RestoreRecoveryReceiptProjection | null>(null);

  const restore = $derived(snapshot?.restore ?? null);
  const canInspect = $derived(snapshot?.capabilities.includes("restoreInspection") === true);
  const canExecute = $derived(snapshot?.capabilities.includes("restoreExecution") === true);
  const canExecuteAdapter = $derived(
    snapshot?.capabilities.includes("restoreAdapterExecution") === true,
  );
  const canRecover = $derived(snapshot?.capabilities.includes("restoreRecovery") === true);
  const blockedByRestore = $derived(
    restore?.state === "active" || restore?.state === "recoveryRequired",
  );
  const allChoicesMade = $derived(
    inspection !== null &&
      inspection.domains.every((domain) => choices[domain.domainId] === "useArchive" ||
        choices[domain.domainId] === "keepCurrent") &&
      inspection.domains.some((domain) => choices[domain.domainId] === "useArchive"),
  );
  const archiveOptions = $derived<SelectOption[]>([
    { value: "hostPicker", label: "Choose archive…" },
    ...(snapshot?.backup?.inventory.archives.map((archive) => ({
      value: archive.archiveSha256,
      label: `${archive.createdAt} — ${archive.archiveId}`,
    })) ?? []),
  ]);
  const conflictOptions: RadioGroupOption[] = [
    { value: "useArchive", label: "Use archive" },
    { value: "keepCurrent", label: "Keep current" },
  ];

  onMount(() => {
    if (snapshot === null) void refresh();
  });

  async function refresh(): Promise<void> {
    await run("idle", async () => {
      acceptSnapshot(
        await client.snapshot({
          protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
          requestId: nextRequestId(),
        }),
      );
    });
  }

  async function inspect(): Promise<void> {
    clearResult();
    await run("inspecting", async () => {
      const outcome = await client.inspectRestore({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
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
        error = `Archive locked: ${outcome.detail}`;
      } else {
        reject(outcome.rejection);
      }
    });
  }

  async function createPlan(): Promise<void> {
    if (inspection === null || inspectionGeneration === null || !allChoicesMade) return;
    await run("planning", async () => {
      const domainChoices: RestoreDomainChoice[] = inspection!.domains.map((domain) => ({
        domainId: domain.domainId,
        choice: choices[domain.domainId] as Exclude<RestoreChoice, "">,
      }));
      const outcome = await client.planRestore({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
        generation: inspectionGeneration!,
        archiveSha256: inspection!.archiveSha256,
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
    if (plan === null || planGeneration === null) return;
    await run("publishing", async () => {
      const outcome = await client.executeRestore({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
        generation: planGeneration!,
        confirmationDigest: plan!.confirmationDigest,
      });
      if (outcome.status === "succeeded") {
        execution = outcome.receipt;
        acceptSnapshot(outcome.snapshot);
        notice = "Restore published and verified.";
      } else if (outcome.status === "rolledBack") {
        failure = outcome.failure;
        acceptSnapshot(outcome.snapshot);
        notice = "Restore failed and the host verified rollback.";
      } else if (outcome.status === "recoveryRequired") {
        failure = outcome.failure;
        acceptSnapshot(outcome.snapshot);
        error = "Restore could not establish rollback. Recovery is required.";
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
    if (inspection === null || inspectionGeneration === null) return;
    await run("publishing", async () => {
      const outcome = await client.executeAdapterRestore({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
        generation: inspectionGeneration!,
        archiveSha256: inspection!.archiveSha256,
        domainId,
        confirmationDigest,
        requirement,
      });
      if (outcome.status === "completed") {
        adapterReceipt = outcome.receipt;
        acceptSnapshot(outcome.snapshot);
        notice = `Adapter restore finished with ${outcome.receipt.outcome}.`;
      } else {
        reject(outcome.rejection);
      }
    });
  }

  function executeInspectedAdapter(
    domain: RestoreDomainInspectionProjection,
  ): Promise<void> {
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
    await run("recovering", async () => {
      const outcome = await client.recoverRestore({
        protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION,
        requestId: nextRequestId(),
      });
      if (outcome.status === "recovered") {
        recoveryReceipt = outcome.receipt;
        acceptSnapshot(outcome.snapshot);
        notice = `Recovery finished with ${outcome.receipt.outcome}.`;
      } else if (outcome.status === "recoveryRequired") {
        failure = outcome.failure;
        acceptSnapshot(outcome.snapshot);
        error = "Recovery remains required.";
      } else {
        reject(outcome.rejection);
      }
    });
  }

  async function run(nextActivity: Activity, action: () => Promise<void>): Promise<void> {
    activity = nextActivity;
    error = null;
    notice = null;
    try {
      await action();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      activity = "idle";
    }
  }

  function choose(domainId: string, choice: string): void {
    choices = { ...choices, [domainId]: choice as RestoreChoice };
    plan = null;
    planGeneration = null;
  }

  function reject(rejection: ConfigOperationRejection): void {
    error = `${rejection.code}: ${rejection.detail}`;
    if (rejection.snapshot) acceptSnapshot(rejection.snapshot);
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

  function acceptSnapshot(value: ConfigOperationsSnapshot): void {
    snapshot = value;
    onSnapshot?.(value);
  }
</script>

<div class="longhorn-config-page" aria-busy={activity !== "idle"}>
  {#if error}
    <Callout tone="danger" title="Restore operation failed" message={error} announceMode="assertive" />
  {/if}
  {#if notice}
    <Callout tone="success" title="Restore operation complete" message={notice} announceMode="polite" />
  {/if}

  {#if restore === null}
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
      <div class="longhorn-config-flow">
        <label for="longhorn-restore-archive">Backup archive</label>
        <Select
          id="longhorn-restore-archive"
          value={selection}
          options={archiveOptions}
          native={true}
          disabled={activity !== "idle" || !canInspect}
          onValueChange={(value) => (selection = value)}
        />
        <Button
          variant="secondary"
          loading={activity === "inspecting"}
          disabled={activity !== "idle" || !canInspect}
          onClick={() => void inspect()}
        >
          Inspect archive
        </Button>
      </div>
      <p>Paths, unlock material, and archive bytes remain inside host authority.</p>
    </Surface>

    {#if inspection}
      <Surface asRole="region" label="Verified archive inspection">
        <h3>Verified archive</h3>
        <div class="longhorn-config-details">
          <DetailItem label="Archive" value={inspection.archiveId} />
          <DetailItem label="Created" value={inspection.createdAt} />
          <DetailItem label="Integrity" value={RESTORE_INTEGRITY_LABELS[inspection.integrity]} />
          <DetailItem label="Authenticity" value={RESTORE_AUTHENTICITY_LABELS[inspection.authenticity]} />
          <DetailItem label="Application identity" value={identityLabel(inspection.identity.application)} />
          <DetailItem label="Producer identity" value={identityLabel(inspection.identity.producer)} />
          <DetailItem label="Archive digest" value={inspection.archiveSha256} truncateValue={true} />
        </div>
      </Surface>

      <section aria-label="Restore conflict choices">
        <h3>Domain choices</h3>
        {#each inspection.domains as domain (domain.domainId)}
          <Surface asRole="group" label={domain.domainId}>
            <div class="longhorn-config-domain">
              <div>
                <strong>{domain.domainId}</strong>
                <p>{compatibilityLabel(domain.compatibility)}</p>
                <small>{domain.storageClass} · {domain.consistencyGroup} · {domain.sourceState}</small>
              </div>
              <RadioGroup
                ariaLabel={`Restore choice for ${domain.domainId}`}
                value={choices[domain.domainId]}
                options={conflictOptions.map((option) => ({
                  ...option,
                  disabled: option.value === "useArchive" && !canUseArchive(domain),
                }))}
                disabled={activity !== "idle"}
                onValueChange={(value) => choose(domain.domainId, value)}
              />
            </div>
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
          <ul>
            {#each inspection.consistencyGroups as group (group.id)}
              <li><strong>{group.id}</strong>: {group.mode} via {group.authority}</li>
            {/each}
          </ul>
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
          loading={activity === "planning"}
          disabled={activity !== "idle" || !allChoicesMade}
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
        <ul>
          {#each plan.entries as entry (entry.domainId)}
            <li>
              <strong>{entry.domainId}</strong>: {entry.choice}
              {#if entry.action} → {entry.action}{/if}
              {#if entry.current?.state === "present"}
                ({entry.current.byteLength} bytes, {entry.current.sha256})
              {/if}
            </li>
          {/each}
        </ul>
        <p class="longhorn-config-digest">{plan.confirmationDigest}</p>
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

  {#if activity === "publishing"}
    <Callout
      tone="warning"
      title="Host-owned publication in progress"
      message="Closing this view does not cancel staging, safety backup, publication, rollback, or recovery."
      announceMode="assertive"
    />
  {:else if activity === "recovering"}
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
</div>

<style>
  .longhorn-config-page,
  .longhorn-config-details,
  .longhorn-config-flow {
    display: grid;
    gap: 0.75rem;
  }

  .longhorn-config-details {
    grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
  }

  .longhorn-config-domain {
    align-items: start;
    display: grid;
    gap: 1rem;
    grid-template-columns: minmax(0, 1fr) minmax(12rem, auto);
  }

  .longhorn-config-domain p {
    margin-block: 0.25rem;
  }

  .longhorn-config-digest,
  small {
    overflow-wrap: anywhere;
  }
</style>
