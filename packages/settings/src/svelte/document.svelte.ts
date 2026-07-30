import type { SettingsClient } from "../client.ts";
import type {
  SettingsApplyUnitDefinition,
  SettingsPageDefinition,
  SettingsRegistrySnapshot,
  SettingsRequestId,
  SettingsScopeId,
  SettingsScopeSnapshot,
} from "../generated/protocol.ts";
import {
  projectSettingsRegistry,
  resolveSettingsDeepLink,
  type SettingsRegistryProjection,
} from "../registry.ts";
import {
  MissingSettingsRendererError,
  SettingsRegistryUnavailableError,
  SettingsScopeNotLoadedError,
} from "./errors.ts";
import {
  SettingsPageSession,
  type SettingsPageSessionHost,
} from "./page.svelte.ts";
import { SettingsScopeState } from "./scope.svelte.ts";
import type {
  SettingsPageRenderer,
  SettingsRendererResolver,
  SettingsRoute,
} from "./types.ts";

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
  #focusRevision = $state(0);
  #renderers = new Map<string, SettingsPageRenderer>();
  #scopes = new Map<SettingsScopeId, SettingsScopeState>();
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

  currentScopes(): SettingsScopeState[] {
    return (
      this.currentPage?.readableScopeIds.flatMap((scopeId) => {
        const scope = this.#scopes.get(scopeId);
        return scope === undefined ? [] : [scope];
      }) ?? []
    );
  }

  scopeSnapshot(scopeId: SettingsScopeId): SettingsScopeSnapshot | undefined {
    return this.#scopes.get(scopeId)?.snapshot;
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
    if (projection.pages.length === 0) {
      return "unsupported";
    }
    const route =
      preferredRoute !== undefined &&
      registry.pages.some(({ id }) => id === preferredRoute.pageId)
        ? preferredRoute
        : { pageId: projection.pages[0]!.id };
    await this.installRoute(route);
    return "ready";
  }

  async installRoute(route: SettingsRoute): Promise<void> {
    const registry = this.requiredRegistry();
    const { page } = resolveSettingsDeepLink(registry, route);
    await Promise.all(
      page.readableScopeIds.map((scopeId) =>
        this.#scopeState(registry, scopeId).start(),
      ),
    );
    this.#route = route;
    this.#pageSession(page);
    this.#focusRevision += 1;
  }

  async clearAuthority(): Promise<void> {
    for (const page of this.#pages.values()) {
      page.stop();
    }
    this.#pages.clear();
    const scopes = [...this.#scopes.values()];
    this.#scopes.clear();
    await Promise.all(scopes.map((scope) => scope.stop()));
  }

  async stop(): Promise<void> {
    await this.clearAuthority();
    this.#registry = undefined;
    this.#projection = undefined;
    this.#route = undefined;
    this.#renderers.clear();
  }

  requiredRegistry(): SettingsRegistrySnapshot {
    if (this.#registry === undefined) {
      throw new SettingsRegistryUnavailableError();
    }
    return this.#registry;
  }

  #scopeState(
    registry: SettingsRegistrySnapshot,
    scopeId: SettingsScopeId,
  ): SettingsScopeState {
    const existing = this.#scopes.get(scopeId);
    if (existing !== undefined) return existing;
    const state = new SettingsScopeState({
      client: this.#options.client,
      registry,
      scopeId,
      nextRequestId: this.#options.nextRequestId,
      onFailure: (failure) => this.#options.onScopeFailure(failure.error),
    });
    this.#scopes.set(scopeId, state);
    return state;
  }

  #pageSession(page: SettingsPageDefinition): SettingsPageSession {
    const existing = this.#pages.get(page.id);
    if (existing !== undefined) return existing;
    const host: SettingsPageSessionHost = {
      client: this.#options.client,
      registry: () => this.requiredRegistry(),
      scope: (unit) => this.#requiredScope(unit),
      nextRequestId: this.#options.nextRequestId,
      onError: this.#options.onMutationError,
    };
    const session = new SettingsPageSession(page, host);
    this.#pages.set(page.id, session);
    return session;
  }

  #requiredScope(unit: SettingsApplyUnitDefinition): SettingsScopeState {
    const scope = this.#scopes.get(unit.scopeId);
    if (scope === undefined) {
      throw new SettingsScopeNotLoadedError(unit.scopeId);
    }
    return scope;
  }
}
