<script lang="ts">
  import {
    Button,
    Callout,
    Dialog,
    FormActions,
    SettingsShell as PoodleSettingsShell,
  } from "@inflatable-cookie/poodle-svelte";
  import type {
    SettingsRecoveryCode,
    SettingsRejectionCode,
  } from "@inflatable-cookie/longhorn/settings/protocol";
  import { tick } from "svelte";

  import { errorMessage, reportFailure } from "../../failures.ts";
  import { useSettingsSession } from "../svelte.ts";
  import type { SettingsRendererResolver, SettingsSession } from "../svelte.ts";

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

  const recoveryMessages: Record<SettingsRecoveryCode, string> = {
    corrupt: "The saved settings could not be read.",
    futureSchema: "These settings were saved by a newer version of the app.",
    authorityUnavailable: "The app can’t access these settings right now.",
    recoveryInProgress: "Wait for settings recovery to finish before making changes.",
    recoveryRequired: "Recover these settings before making changes.",
  };

  const rejectionMessages: Record<SettingsRejectionCode, string> = {
    invalidIntent: "Review the values in your draft before trying again.",
    policyBlocked: "This change is blocked by policy.",
    readOnly: "This setting is read-only.",
    hidden: "This setting is no longer available on this page.",
    unsupported: "This change is unavailable in the current setup.",
    recoveryRequired: "Recover these settings before making changes.",
    unauthorized: "You don’t have permission to make this change.",
    registryChanged: "The available settings changed. Reopen the page and review your draft.",
  };

  let searchQuery = $state("");
  let focusTarget = $state<HTMLElement | null>(null);

  useSettingsSession(
    () => session,
    () => resolveRenderer,
  );

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
  const navigation = $derived.by(() => {
    const routes = new Map<string, { pageId: string; anchorId?: string }>();
    const query = searchQuery.trim();
    const matches = query ? session.search(query) : undefined;
    const anchorsByPage = new Map<string, Array<{ id: string; label: string }>>();
    if (matches) {
      for (const { page, anchor } of matches) {
        if (!anchor) {
          continue;
        }
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

    const groups =
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
      ) ?? [];
    return { groups, routes };
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
      session.registry?.applyUnits.some((unit) => unit.id === unitId && unit.timing === "staged"),
    ) ?? false,
  );

  // Poodle's shell refuses to close while this is set, and it reads it at the
  // moment of the attempt. So this has to answer "would a close be refused",
  // not "was one refused" -- a guard raised by our own handler arrives too
  // late, after the shell has already decided.
  const closeRefusedReason = $derived(
    session.canLeaveCurrent ? null : "Apply or discard this page before leaving.",
  );

  let focusedRoute: { session: SettingsSession; revision: number } | undefined;
  $effect(() => {
    const current = session;
    const revision = current.focusRevision;
    const target = focusTarget;
    if (!open || target === null || revision === 0) {
      return;
    }
    void tick().then(() => {
      if (
        target.isConnected &&
        target === focusTarget &&
        current === session &&
        revision === current.focusRevision &&
        open &&
        (focusedRoute?.session !== current || focusedRoute.revision !== revision)
      ) {
        focusedRoute = { session: current, revision };
        target.focus();
      }
    });
  });

  // A default rather than an optional parameter: the packed artifact strips
  // type annotations but leaves a bare `anchorId?`, which is not valid JS and
  // fails the composition proof's bundler.
  //
  // The query is deliberately left alone. A narrowed rail is a filtered list
  // of destinations, and clearing it on the first pick would throw away the
  // filter the moment it became useful.
  function navigate(value: string, _anchorId: string | null = null): void {
    const route = navigation.routes.get(value) ?? { pageId: value };
    // A rail click is a shell action like any other: a rejected navigation --
    // a scope that will not load, a deep link that no longer resolves -- is a
    // failure the operator has to see, not a promise nobody observes. Focus
    // moves only after the route it moves to has committed.
    runAction(() => session.navigate(route));
  }

  // Poodle's shell holds itself open on `closeRefusedReason` and returns
  // before it calls `onOpenChange`, so this is the only signal that a refused
  // close was attempted. `requestClose` raises the guard dialog below and
  // returns false; the shell has already refused to close.
  function handleRequestClose(): void {
    if (!session.canLeaveCurrent) {
      session.requestClose();
    }
  }

  // Shell actions (apply, guard resolution, reset, rail navigation) can fail
  // after the session has already moved on, so the rejection is kept here and
  // shown. It sits above the page region rather than inside it: a failed
  // navigation can leave the session in its failed state, and the page that
  // would have carried the record is then not the one on screen. The open
  // dialogs cover the page, so they carry it too.
  let actionFailure = $state<{ readonly error: unknown } | undefined>();

  function runAction<Result>(action: () => Promise<Result>): void {
    actionFailure = undefined;
    let result: Promise<Result>;
    try {
      result = action();
    } catch (error) {
      actionFailure = { error };
      return;
    }
    void result.catch((error: unknown) => {
      actionFailure = { error };
    });
  }

  function handleOpenChange(next: boolean): void {
    if (next) {
      open = true;
      onOpenChange?.(true);
      return;
    }
    // `handleRequestClose` has already raised the guard for a refused close,
    // and Poodle's shell does not reach here in that case.
    if (!session.requestClose()) {
      return;
    }
    open = false;
    onOpenChange?.(false);
  }
</script>

<PoodleSettingsShell
  bind:open
  {title}
  {ariaLabel}
  groups={navigation.groups}
  activePageId={session.route?.pageId ?? null}
  pageTitle={currentPage?.label ?? null}
  bind:searchQuery
  {closeRefusedReason}
  onNavigate={navigate}
  onRequestClose={handleRequestClose}
  onOpenChange={handleOpenChange}
>
  {#snippet page()}
    {#if actionFailure}
      <Callout
        tone="danger"
        title="Settings action failed"
        message={errorMessage(actionFailure.error)}
        announceMode="assertive"
      />
    {/if}
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
        message={errorMessage(session.status.error)}
        announceMode="assertive"
      >
        {#snippet actions()}
          <Button variant="secondary" onClick={() => void session.reconnect().catch(reportFailure)}>
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
            message={recoveryMessages[session.recovery.code]}
            announceMode="assertive"
          />
        {/if}

        {#if pendingActivation.length > 0}
          <Callout
            tone="info"
            title="Activation required"
            message="Some saved changes still need to be activated."
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
            message={rejectionMessages[unitStatus.rejection.code]}
            announceMode="assertive"
          />
        {:else if unitStatus.kind === "failed"}
          <Callout
            tone="danger"
            title="Save failed"
            message={errorMessage(unitStatus.error)}
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
              onClick={() => runAction(() => session.applyCurrent())}
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
  description={actionFailure
    ? errorMessage(actionFailure.error)
    : "Apply or discard this page before leaving."}
  dismissOnBackdrop={false}
  onOpenChange={(next) => {
    if (!next) runAction(() => session.resolveGuard("stay"));
  }}
>
  {#snippet actions()}
    <Button variant="ghost" onClick={() => runAction(() => session.resolveGuard("stay"))}>
      Stay
    </Button>
    <Button
      variant="secondary"
      disabled={session.busy}
      onClick={() => runAction(() => session.resolveGuard("discard"))}
    >
      Discard
    </Button>
    <Button
      variant="primary"
      loading={session.busy}
      disabled={!session.canApplyCurrent}
      onClick={() => runAction(() => session.resolveGuard("apply"))}
    >
      Apply
    </Button>
  {/snippet}
</Dialog>

<Dialog
  open={session.resetRequest !== undefined}
  role="alertdialog"
  title="Reset settings"
  description={actionFailure
    ? errorMessage(actionFailure.error)
    : "Remove the selected user overrides?"}
  dismissOnBackdrop={false}
  onOpenChange={(next) => {
    if (!next) runAction(() => session.resolveReset(false));
  }}
>
  {#snippet actions()}
    <Button variant="secondary" onClick={() => runAction(() => session.resolveReset(false))}>
      Cancel
    </Button>
    <Button variant="primary" onClick={() => runAction(() => session.resolveReset(true))}>
      Reset
    </Button>
  {/snippet}
</Dialog>
