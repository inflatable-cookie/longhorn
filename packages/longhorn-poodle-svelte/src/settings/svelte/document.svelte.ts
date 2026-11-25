import type { SettingsClient } from "@inflatable-cookie/longhorn/settings";
import type {
  SettingsApplyUnitDefinition,
  SettingsPageDefinition,
  SettingsPageId,
  SettingsRegistrySnapshot,
  SettingsRequestId,
  SettingsScopeId,
  SettingsScopeSnapshot,
} from "@inflatable-cookie/longhorn/settings/protocol";
import {
  projectSettingsRegistry,
  resolveSettingsDeepLink,
} from "@inflatable-cookie/longhorn/settings";
import type { SettingsRegistryProjection } from "@inflatable-cookie/longhorn/settings";
import { throwTeardownFailures } from "../../bindings/teardown.ts";
import {
  MissingSettingsRendererError,
  SettingsRegistryUnavailableError,
  SettingsScopeNotLoadedError,
} from "./errors.ts";
import { SettingsPageSession } from "./page.svelte.ts";
import type { SettingsPageSessionHost } from "./page.svelte.ts";
import { SettingsScopeState } from "./scope.svelte.ts";
import type { SettingsPageRenderer, SettingsRendererResolver, SettingsRoute } from "./types.ts";

export interface SettingsDocumentOptions {
  readonly client: SettingsClient;
  readonly nextRequestId: () => SettingsRequestId;
  readonly onScopeFailure: (error: unknown) => void;
  readonly onMutationError: (error: unknown) => void;
}

export class SettingsSessionDocument {
  readonly #options: SettingsDocumentOptions;
  #registry = $state.raw<SettingsRegistrySnapshot | undefined>(undefined);
  #projection = $state.raw<SettingsRegistryProjection | undefined>(undefined);
  #route = $state.raw<SettingsRoute | undefined>(undefined);
  #routeGeneration = 0;
  #focusRevision = $state(0);
  #renderers = new Map<string, SettingsPageRenderer>();
  #scopes = new Map<SettingsPageId, Map<SettingsScopeId, SettingsScopeState>>();
  #pages = new Map<string, SettingsPageSession>();

  constructor(options: SettingsDocumentOptions) {
    this.#options = options;
  }

  get registry(): SettingsRegistrySnapshot | undefined {
    return this.#registry;
  }

  get projection(): SettingsRegistryProjection | undefined {
    return this.#projection;
  }

  get route(): SettingsRoute | undefined {
    return this.#route;
  }

  get focusRevision(): number {
    return this.#focusRevision;
  }

  get currentPage(): SettingsPageDefinition | undefined {
    const pageId = this.#route?.pageId;
    return this.#registry?.pages.find(({ id }) => id === pageId);
  }

  get currentRenderer(): SettingsPageRenderer | undefined {
    const page = this.currentPage;
    return page === undefined ? undefined : this.#renderers.get(page.id);
  }

  get currentPageSession(): SettingsPageSession | undefined {
    const page = this.currentPage;
    return page === undefined ? undefined : this.#pageSession(page);
  }

  currentScopes(): Array<SettingsScopeState> {
    return [...(this.#currentScopes()?.values() ?? [])];
  }

  scopeSnapshot(scopeId: SettingsScopeId): SettingsScopeSnapshot | undefined {
    return this.#currentScopes()?.get(scopeId)?.snapshot;
  }

  #currentScopes(): Map<SettingsScopeId, SettingsScopeState> | undefined {
    const pageId = this.#route?.pageId;
    return pageId === undefined ? undefined : this.#scopes.get(pageId);
  }

  async installRegistry(
    registry: SettingsRegistrySnapshot,
    resolver: SettingsRendererResolver,
    preferredRoute?: SettingsRoute,
  ): Promise<"ready" | "unsupported"> {
    const renderers = new Map<string, SettingsPageRenderer>();
    for (const page of registry.pages) {
      const renderer = resolver(page.rendererId, page);
      if (renderer === undefined) {
        throw new MissingSettingsRendererError(page);
      }
      renderers.set(page.id, renderer);
    }
    const projection = projectSettingsRegistry(registry);
    this.#registry = registry;
    this.#projection = projection;
    this.#renderers = renderers;
    const [firstPage] = projection.pages;
    if (firstPage === undefined) {
      return "unsupported";
    }
    const route =
      preferredRoute !== undefined && registry.pages.some(({ id }) => id === preferredRoute.pageId)
        ? preferredRoute
        : { pageId: firstPage.id };
    await this.installRoute(route);
    return "ready";
  }

  async installRoute(route: SettingsRoute): Promise<void> {
    const registry = this.requiredRegistry();
    const { page } = resolveSettingsDeepLink(registry, route);
    // Scopes start asynchronously, so a route with scopes can commit after a
    // later navigation that had none. A renderer-only page exposes it: it
    // commits immediately, then the scopes of the page it was navigated away
    // from finish and overwrite it. Last navigation wins, not last scope to
    // settle.
    const generation = ++this.#routeGeneration;
    await Promise.all(
      page.readableScopeIds.map((scopeId) => this.#scopeState(registry, page.id, scopeId).start()),
    );
    if (generation !== this.#routeGeneration) {
      return;
    }
    this.#route = route;
    this.#pageSession(page);
    this.#focusRevision += 1;
  }

  async clearAuthority(): Promise<void> {
    // Any route install still awaiting its scope starts is superseded.
    this.#routeGeneration += 1;
    for (const page of this.#pages.values()) {
      page.stop();
    }
    this.#pages.clear();
    const scopes = [...this.#scopes.values()].flatMap((pageScopes) => [...pageScopes.values()]);
    this.#scopes.clear();
    throwTeardownFailures(await Promise.allSettled(scopes.map((scope) => scope.stop())));
  }

  async stop(): Promise<void> {
    const cleanup = this.clearAuthority();
    this.#registry = undefined;
    this.#projection = undefined;
    this.#route = undefined;
    this.#renderers.clear();
    await cleanup;
  }

  requiredRegistry(): SettingsRegistrySnapshot {
    if (this.#registry === undefined) {
      throw new SettingsRegistryUnavailableError();
    }
    return this.#registry;
  }

  #scopeState(
    registry: SettingsRegistrySnapshot,
    pageId: SettingsPageId,
    scopeId: SettingsScopeId,
  ): SettingsScopeState {
    let pageScopes = this.#scopes.get(pageId);
    if (pageScopes === undefined) {
      pageScopes = new Map();
      this.#scopes.set(pageId, pageScopes);
    }
    const existing = pageScopes.get(scopeId);
    if (existing !== undefined) {
      return existing;
    }
    const state = new SettingsScopeState({
      client: this.#options.client,
      registry,
      pageId,
      scopeId,
      nextRequestId: this.#options.nextRequestId,
      onFailure: (failure) => this.#options.onScopeFailure(failure.error),
    });
    pageScopes.set(scopeId, state);
    return state;
  }

  #pageSession(page: SettingsPageDefinition): SettingsPageSession {
    const existing = this.#pages.get(page.id);
    if (existing !== undefined) {
      return existing;
    }
    const host: SettingsPageSessionHost = {
      client: this.#options.client,
      registry: () => this.requiredRegistry(),
      scope: (unit) => this.#requiredScope(page.id, unit),
      nextRequestId: this.#options.nextRequestId,
      onError: this.#options.onMutationError,
    };
    const session = new SettingsPageSession(page, host);
    this.#pages.set(page.id, session);
    return session;
  }

  #requiredScope(pageId: SettingsPageId, unit: SettingsApplyUnitDefinition): SettingsScopeState {
    const scope = this.#scopes.get(pageId)?.get(unit.scopeId);
    if (scope === undefined) {
      throw new SettingsScopeNotLoadedError(unit.scopeId);
    }
    return scope;
  }
}
