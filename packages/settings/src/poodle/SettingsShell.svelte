<script lang="ts">
  import {
    Button,
    Callout,
    Dialog,
    FormActions,
    PageHeader,
    SidebarNav,
    Surface,
    TextInput,
  } from "@poodle/svelte";
  import { tick } from "svelte";
  import type { SidebarNavGroup } from "@poodle/svelte";

  import {
    useSettingsSession,
    type SettingsHostForm,
    type SettingsRendererResolver,
    type SettingsSession,
  } from "../svelte.ts";

  interface Props {
    session: SettingsSession;
    resolveRenderer: SettingsRendererResolver;
    host?: SettingsHostForm;
    open?: boolean;
    title?: string;
    ariaLabel?: string;
    onOpenChange?: (open: boolean) => void;
  }

  let {
    session,
    resolveRenderer,
    host = "window",
    open = $bindable(true),
    title = "Settings",
    ariaLabel = "Settings",
    onOpenChange,
  }: Props = $props();

  let searchQuery = $state("");
  let focusTarget = $state<HTMLElement | null>(null);

  useSettingsSession(session, resolveRenderer);

  const navigationGroups = $derived<SidebarNavGroup[]>(
    session.navigation?.modules.flatMap(({ module, sections }) =>
      sections.map(({ section, pages }) => ({
        id: section.id,
        label:
          session.navigation!.modules.length > 1
            ? `${module.label} · ${section.label}`
            : section.label,
        items: pages.map((page) => ({
          value: page.id,
          label: page.label,
        })),
      })),
    ) ?? [],
  );
  const searchResults = $derived(session.search(searchQuery));
  const page = $derived(session.currentPage);
  const renderer = $derived(session.currentRenderer);
  const context = $derived(session.currentContext);
  const pendingActivation = $derived(
    session.activationRequirements.filter(({ state }) => state === "pending"),
  );
  const unitStatus = $derived(session.primaryUnitStatus);
  const hasStagedUnit = $derived(
    page?.writableApplyUnitIds.some((unitId) =>
      session.registry?.applyUnits.some(
        (unit) => unit.id === unitId && unit.timing === "staged",
      ),
    ) ?? false,
  );

  $effect(() => {
    session.focusRevision;
    void tick().then(() => focusTarget?.focus());
  });

  function navigate(pageId: string): void {
    void session.navigate({ pageId });
  }

  function navigateSearch(index: number): void {
    const result = searchResults[index];
    if (result === undefined) return;
    void session
      .navigate({
        pageId: result.page.id,
        anchorId: result.anchor?.id,
      })
      .then(async (navigated) => {
        if (navigated) {
          await tick();
          focusTarget?.focus();
        }
      });
  }

  function requestClose(): void {
    if (session.requestClose()) {
      open = false;
      onOpenChange?.(false);
    }
  }

  function handleDialogOpen(next: boolean): void {
    if (next) {
      open = true;
      onOpenChange?.(true);
      return;
    }
    if (!session.requestClose()) {
      open = true;
      return;
    }
    open = false;
    onOpenChange?.(false);
  }
</script>

{#snippet shellContent()}
  <div class="longhorn-settings-shell" data-host={host}>
    <div class="longhorn-settings-shell__search">
      <TextInput
        value={searchQuery}
        type="search"
        placeholder="Search settings"
        ariaLabel="Search settings"
        showClearButton={true}
        onValueChange={(value) => (searchQuery = value)}
      />
      {#if searchQuery.trim() && searchResults.length > 0}
        <ul aria-label="Settings search results">
          {#each searchResults as result, index (`${result.kind}:${result.page.id}:${result.anchor?.id ?? ""}`)}
            <li>
              <Button
                variant="ghost"
                onClick={() => navigateSearch(index)}
              >
                {result.page.label}{result.anchor?.label
                  ? ` · ${result.anchor.label}`
                  : ""}
              </Button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <aside class="longhorn-settings-shell__navigation">
      <SidebarNav
        groups={navigationGroups}
        value={session.route?.pageId ?? null}
        ariaLabel="Settings pages"
        onValueChange={navigate}
      />
    </aside>

    <main class="longhorn-settings-shell__page">
      {#if session.status.kind === "loading"}
        <Callout
          tone="pending"
          title="Loading settings"
          announceMode="polite"
        />
      {:else if session.status.kind === "reconnecting"}
        <Callout
          tone="pending"
          title="Reconnecting settings"
          announceMode="polite"
        />
      {:else if session.status.kind === "unsupported"}
        <Callout
          tone="warning"
          title="Settings unavailable"
          message={session.status.reason}
          announceMode="polite"
        />
      {:else if session.status.kind === "failed"}
        <Callout
          tone="danger"
          title="Settings failed"
          message={String(session.status.error)}
          announceMode="assertive"
        >
          {#snippet actions()}
            <Button
              variant="secondary"
              onClick={() => void session.reconnect().catch(() => undefined)}
            >
              Retry
            </Button>
          {/snippet}
        </Callout>
      {:else if page && renderer && context}
        <section
          bind:this={focusTarget}
          class="longhorn-settings-shell__page-focus"
          tabindex="-1"
          data-page-id={page.id}
          data-anchor-id={session.route?.anchorId}
          aria-label={session.route?.anchorId
            ? `${page.label}: ${session.route.anchorId}`
            : page.label}
        >
          <PageHeader title={page.label} level={2}>
            {#snippet actions()}
              <Button variant="ghost" onClick={requestClose}>Close</Button>
            {/snippet}
          </PageHeader>

          {#if session.recovery}
            <Callout
              tone="danger"
              title="Recovery required"
              message={session.recovery.code}
              announceMode="assertive"
            />
          {/if}

          {#if pendingActivation.length > 0}
            <Callout
              tone="info"
              title="Activation required"
              message={pendingActivation
                .map(({ targetId }) => targetId)
                .join(", ")}
              announceMode="polite"
            />
          {/if}

          {#if unitStatus.kind === "pending"}
            <Callout
              tone="pending"
              title="Saving"
              announceMode="polite"
            />
          {:else if unitStatus.kind === "saved"}
            <Callout
              tone="success"
              title="Saved"
              announceMode="polite"
            />
          {:else if unitStatus.kind === "conflict"}
            <Callout
              tone="warning"
              title="Settings changed elsewhere"
              message="Your draft is preserved. Review the current values before applying again."
              announceMode="assertive"
            />
          {:else if unitStatus.kind === "rejected"}
            <Callout
              tone="danger"
              title="Change rejected"
              message={unitStatus.rejection.code}
              announceMode="assertive"
            />
          {:else if unitStatus.kind === "failed"}
            <Callout
              tone="danger"
              title="Save failed"
              message={String(unitStatus.error)}
              announceMode="assertive"
            />
          {/if}

          <div class="longhorn-settings-shell__consumer-page">
            {@render renderer(context)}
          </div>

          {#if hasStagedUnit}
            <FormActions>
              <Button
                variant="secondary"
                disabled={!session.dirty || session.busy}
                onClick={() => session.cancelCurrent()}
              >
                Cancel
              </Button>
              <Button
                variant="primary"
                loading={session.busy}
                disabled={!session.canApplyCurrent}
                onClick={() =>
                  void session.applyCurrent().catch(() => undefined)}
              >
                Apply
              </Button>
            </FormActions>
          {/if}
        </section>
      {/if}
    </main>
  </div>

  <Dialog
    open={session.guard !== undefined}
    role="alertdialog"
    title="Unsaved changes"
    description="Apply or discard this page before leaving."
    dismissOnBackdrop={false}
    onOpenChange={(next) => {
      if (!next) void session.resolveGuard("stay").catch(() => undefined);
    }}
  >
    {#snippet actions()}
      <Button
        variant="ghost"
        onClick={() =>
          void session.resolveGuard("stay").catch(() => undefined)}
      >
        Stay
      </Button>
      <Button
        variant="secondary"
        disabled={session.busy}
        onClick={() =>
          void session.resolveGuard("discard").catch(() => undefined)}
      >
        Discard
      </Button>
      <Button
        variant="primary"
        loading={session.busy}
        disabled={!session.canApplyCurrent}
        onClick={() =>
          void session.resolveGuard("apply").catch(() => undefined)}
      >
        Apply
      </Button>
    {/snippet}
  </Dialog>

  <Dialog
    open={session.resetRequest !== undefined}
    role="alertdialog"
    title="Reset settings"
    description="Remove the selected user overrides?"
    dismissOnBackdrop={false}
    onOpenChange={(next) => {
      if (!next) void session.resolveReset(false).catch(() => undefined);
    }}
  >
    {#snippet actions()}
      <Button
        variant="secondary"
        onClick={() =>
          void session.resolveReset(false).catch(() => undefined)}
      >
        Cancel
      </Button>
      <Button
        variant="primary"
        onClick={() =>
          void session.resolveReset(true).catch(() => undefined)}
      >
        Reset
      </Button>
    {/snippet}
  </Dialog>
{/snippet}

{#if host === "modal"}
  <Dialog
    bind:open
    {title}
    {ariaLabel}
    width="xl"
    showCloseButton={true}
    onOpenChange={handleDialogOpen}
  >
    {@render shellContent()}
  </Dialog>
{:else if open}
  <Surface
    tone={host === "window" ? "canvas" : "panel"}
    border={host === "window" ? "none" : "subtle"}
    padding="none"
    asRole="region"
    label={ariaLabel}
  >
    {@render shellContent()}
  </Surface>
{/if}

<style>
  .longhorn-settings-shell {
    display: grid;
    grid-template-columns: minmax(11rem, 15rem) minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr);
    min-block-size: 24rem;
  }

  .longhorn-settings-shell__search {
    grid-column: 1 / -1;
    padding: 0.75rem;
  }

  .longhorn-settings-shell__search ul {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin: 0.5rem 0 0;
    padding: 0;
    list-style: none;
  }

  .longhorn-settings-shell__navigation,
  .longhorn-settings-shell__page {
    min-block-size: 0;
    overflow: auto;
  }

  .longhorn-settings-shell__navigation {
    padding: 0.5rem;
  }

  .longhorn-settings-shell__page {
    padding: 1rem;
  }

  .longhorn-settings-shell__page-focus {
    display: grid;
    gap: 1rem;
    outline: none;
  }

  .longhorn-settings-shell__consumer-page {
    min-inline-size: 0;
  }
</style>
