<script lang="ts">
  import {
    Button,
    Callout,
    Dialog,
    FormActions,
    SettingsShell as PoodleSettingsShell,
  } from "@inflatable-cookie/poodle-svelte";
  import { tick } from "svelte";

  import {
    useSettingsSession,
    type SettingsRendererResolver,
    type SettingsSession,
  } from "../svelte.ts";

  /**
   * Binds a `SettingsSession` to Poodle's `SettingsShell`.
   *
   * Poodle owns the chrome: the dialog, the header and search field, the nav
   * rail, the results list, the page region. It is presentational and takes
   * `groups`, `searchResults` and a `page` snippet — it has no notion of a
   * session, an apply unit, or a renderer, and it may not depend on Longhorn.
   *
   * This file is what closes that gap, and it is binding rather than design.
   * Everything below is session-derived state Poodle cannot know about:
   * connection status, recovery, activation requirements, apply-unit status,
   * the staged apply actions, the unsaved-changes guard, and reset
   * confirmation.
   */
  interface Props {
    session: SettingsSession;
    resolveRenderer: SettingsRendererResolver;
    open?: boolean;
    title?: string;
    /**
     * The dialog's accessible name, when it should differ from the title.
     *
     * "Settings" is the right visible title in every application and the
     * wrong accessible name in all of them: a screen-reader user with three
     * windows open hears it three times. Defaults to `title`.
     */
    ariaLabel?: string | null;
    onOpenChange?: (open: boolean) => void;
  }

  let {
    session,
    resolveRenderer,
    open = $bindable(true),
    title = "Settings",
    ariaLabel = null,
    onOpenChange,
  }: Props = $props();

  let searchQuery = $state("");
  let focusTarget = $state<HTMLElement | null>(null);

  useSettingsSession(() => session, () => resolveRenderer);

  /**
   * Search narrows the nav rail; it does not replace the page.
   *
   * Poodle's shell used to render a separate results list over the page
   * region. It duplicated the nav as a second list and hid the page you were
   * working on behind the list you had just navigated from, so it is gone.
   * Narrowing keeps one list of destinations and leaves the page alone, which
   * is also why the query survives a navigation.
   *
   * Anchor matches become their own entries rather than collapsing into their
   * page. Searching "Output" and landing on the Audio page with no idea which
   * control matched is barely better than not searching.
   */
  const routes = new Map<string, { pageId: string; anchorId?: string }>();

  const navigationGroups = $derived.by(() => {
    routes.clear();
    const query = searchQuery.trim();
    const matches = query ? session.search(query) : undefined;
    const anchorsByPage = new Map<string, { id: string; label: string }[]>();
    if (matches) {
      for (const { page, anchor } of matches) {
        if (!anchor) continue;
        const existing = anchorsByPage.get(page.id) ?? [];
        if (!existing.some(({ id }) => id === anchor.id)) {
          // An anchor may carry no label. It still matched -- on its id -- so
          // it stays in the rail under the id rather than as a blank row.
          existing.push({ id: anchor.id, label: anchor.label ?? anchor.id });
        }
        anchorsByPage.set(page.id, existing);
      }
    }
    const matchedPages = matches && new Set(matches.map(({ page }) => page.id));

    return (
      session.navigation?.modules.flatMap(({ sections }) =>
        sections
          .map(({ section, pages }) => ({
            id: section.id,
            // The section's own label, always. This used to prefix the
            // module's whenever more than one was registered, which reads as
            // "STORAGE · STORAGE & BACKUPS" for a Storage module holding a
            // Storage & Backups section. A host that wants its module named
            // writes that into the section label.
            label: section.label,
            items: pages
              .filter((page) => matchedPages === undefined || matchedPages.has(page.id))
              .flatMap((page) => {
                routes.set(page.id, { pageId: page.id });
                const entry = { value: page.id, label: page.label };
                const anchors = anchorsByPage.get(page.id) ?? [];
                return [
                  entry,
                  ...anchors.map((anchor) => {
                    // A synthetic value rather than a composite string: ids
                    // are host-supplied and a separator could appear in one.
                    const value = `${page.id}\u0000${anchor.id}`;
                    routes.set(value, { pageId: page.id, anchorId: anchor.id });
                    return { value, label: `${page.label} · ${anchor.label}` };
                  }),
                ];
              }),
          }))
          // A section whose every page was filtered out is not a heading over
          // nothing; it is absent.
          .filter(({ items }) => items.length > 0),
      ) ?? []
    );
  });

  const currentPage = $derived(session.currentPage);
  const renderer = $derived(session.currentRenderer);
  const context = $derived(session.currentContext);
  const pendingActivation = $derived(
    session.activationRequirements.filter(({ state }) => state === "pending"),
  );
  const unitStatus = $derived(session.primaryUnitStatus);
  const hasStagedUnit = $derived(
    currentPage?.writableApplyUnitIds.some((unitId) =>
      session.registry?.applyUnits.some(
        (unit) => unit.id === unitId && unit.timing === "staged",
      ),
    ) ?? false,
  );

  // Poodle's shell refuses to close while this is set, and it reads it at the
  // moment of the attempt. So this has to answer "would a close be refused",
  // not "was one refused" -- a guard raised by our own handler arrives too
  // late, after the shell has already decided.
  const closeRefusedReason = $derived(
    session.canLeaveCurrent ? null : "Apply or discard this page before leaving.",
  );

  $effect(() => {
    session.focusRevision;
    void tick().then(() => focusTarget?.focus());
  });

  // A default rather than an optional parameter: the packed artifact strips
  // type annotations but leaves a bare `anchorId?`, which is not valid JS and
  // fails the composition proof's bundler.
  //
  // The query is deliberately left alone. A narrowed rail is a filtered list
  // of destinations, and clearing it on the first pick would throw away the
  // filter the moment it became useful.
  function navigate(value: string, _anchorId: string | null = null): void {
    const route = routes.get(value) ?? { pageId: value };
    void session.navigate(route).then(async (navigated) => {
      if (!navigated) return;
      await tick();
      focusTarget?.focus();
    });
  }

  // Poodle's shell holds itself open on `closeRefusedReason` and returns
  // before it calls `onOpenChange`, so this is the only signal that a refused
  // close was attempted. `requestClose` raises the guard dialog below and
  // returns false; the shell has already refused to close.
  function handleRequestClose(): void {
    if (!session.canLeaveCurrent) session.requestClose();
  }

  function handleOpenChange(next: boolean): void {
    if (next) {
      open = true;
      onOpenChange?.(true);
      return;
    }
    // `handleRequestClose` has already raised the guard for a refused close,
    // and Poodle's shell does not reach here in that case.
    if (!session.requestClose()) return;
    open = false;
    onOpenChange?.(false);
  }
</script>

<PoodleSettingsShell
  bind:open
  {title}
  {ariaLabel}
  groups={navigationGroups}
  activePageId={session.route?.pageId ?? null}
  pageTitle={currentPage?.label ?? null}
  bind:searchQuery
  {closeRefusedReason}
  onNavigate={navigate}
  onRequestClose={handleRequestClose}
  onOpenChange={handleOpenChange}
>
  {#snippet page()}
    {#if session.status.kind === "loading"}
      <Callout tone="pending" title="Loading settings" announceMode="polite" />
    {:else if session.status.kind === "reconnecting"}
      <Callout tone="pending" title="Reconnecting settings" announceMode="polite" />
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
    {:else if currentPage && renderer && context}
      <!-- No page heading. Poodle's nav rail already names the current page
           and its page region carries the label as its accessible name, so a
           second visible title was the duplication the redesign removed. No
           close button either: the dialog has one. -->
      <section
        bind:this={focusTarget}
        class="longhorn-settings-shell__page-focus"
        tabindex="-1"
        data-page-id={currentPage.id}
        data-anchor-id={session.route?.anchorId}
      >
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
            message={pendingActivation.map(({ targetId }) => targetId).join(", ")}
            announceMode="polite"
          />
        {/if}

        {#if unitStatus.kind === "pending"}
          <Callout tone="pending" title="Saving" announceMode="polite" />
        {:else if unitStatus.kind === "saved"}
          <Callout tone="success" title="Saved" announceMode="polite" />
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
              onClick={() => void session.applyCurrent().catch(() => undefined)}
            >
              Apply
            </Button>
          </FormActions>
        {/if}
      </section>
    {/if}
  {/snippet}
</PoodleSettingsShell>

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
      onClick={() => void session.resolveGuard("stay").catch(() => undefined)}
    >
      Stay
    </Button>
    <Button
      variant="secondary"
      disabled={session.busy}
      onClick={() => void session.resolveGuard("discard").catch(() => undefined)}
    >
      Discard
    </Button>
    <Button
      variant="primary"
      loading={session.busy}
      disabled={!session.canApplyCurrent}
      onClick={() => void session.resolveGuard("apply").catch(() => undefined)}
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
      onClick={() => void session.resolveReset(false).catch(() => undefined)}
    >
      Cancel
    </Button>
    <Button
      variant="primary"
      onClick={() => void session.resolveReset(true).catch(() => undefined)}
    >
      Reset
    </Button>
  {/snippet}
</Dialog>
