import type {
  SettingsAnchorDefinition,
  SettingsAnchorId,
  SettingsModuleDefinition,
  SettingsPageDefinition,
  SettingsPageId,
  SettingsRegistrySnapshot,
  SettingsSectionDefinition,
} from "./generated/protocol.ts";

export interface SettingsNavigationModule {
  readonly module: SettingsModuleDefinition;
  readonly sections: ReadonlyArray<SettingsNavigationSection>;
}

export interface SettingsNavigationSection {
  readonly section: SettingsSectionDefinition;
  readonly pages: ReadonlyArray<SettingsPageDefinition>;
}

export interface SettingsRegistryProjection {
  readonly generation: number;
  readonly modules: ReadonlyArray<SettingsNavigationModule>;
  readonly pages: ReadonlyArray<SettingsPageDefinition>;
}

export interface SettingsDeepLink {
  readonly pageId: SettingsPageId;
  readonly anchorId?: SettingsAnchorId;
}

export interface ResolvedSettingsDeepLink {
  readonly page: SettingsPageDefinition;
  readonly anchor: SettingsAnchorDefinition | undefined;
}

export type SettingsSearchMatchKind = "page" | "anchor";

export interface SettingsSearchResult {
  readonly kind: SettingsSearchMatchKind;
  readonly page: SettingsPageDefinition;
  readonly anchor: SettingsAnchorDefinition | undefined;
}

export class SettingsDeepLinkResolutionError extends Error {
  readonly link: SettingsDeepLink;

  constructor(message: string, link: SettingsDeepLink) {
    super(message);
    this.name = "SettingsDeepLinkResolutionError";
    this.link = link;
  }
}

export function projectSettingsRegistry(
  registry: SettingsRegistrySnapshot,
): SettingsRegistryProjection {
  const pagesBySection = new Map<string, Array<SettingsPageDefinition>>();
  for (const page of registry.pages) {
    const pages = pagesBySection.get(page.sectionId) ?? [];
    pages.push(page);
    pagesBySection.set(page.sectionId, pages);
  }
  const sectionsByModule = new Map<string, Array<SettingsNavigationSection>>();
  for (const section of registry.sections) {
    const pages = pagesBySection.get(section.id) ?? [];
    if (pages.length === 0) {
      continue;
    }
    const sections = sectionsByModule.get(section.moduleId) ?? [];
    sections.push({ section, pages });
    sectionsByModule.set(section.moduleId, sections);
  }
  const modules = registry.modules.flatMap((module) => {
    const sections = sectionsByModule.get(module.id) ?? [];
    return sections.length === 0 ? [] : [{ module, sections }];
  });
  return {
    generation: registry.generation,
    modules,
    pages: registry.pages,
  };
}

/**
 * Case folding for search, supplied by the host. This is the TypeScript
 * counterpart of Rust's `HostServices::fold_case`: the host names the locale,
 * and nothing in search guesses one. Used for matching only, never for display.
 */
export interface SettingsSearchHostServices {
  foldCase(value: string): string;
}

export function searchSettingsRegistry(
  registry: SettingsRegistrySnapshot,
  query: string,
  services: SettingsSearchHostServices,
): ReadonlyArray<SettingsSearchResult> {
  const needle = services.foldCase(query.trim());
  if (needle.length === 0) {
    return [];
  }
  const results: Array<SettingsSearchResult> = [];
  for (const page of registry.pages) {
    const pageMatch = [page.label, ...page.keywords]
      .map((value) => services.foldCase(value))
      .some((value) => value.includes(needle));
    if (pageMatch) {
      results.push({ kind: "page", page, anchor: undefined });
    }
    for (const anchor of page.anchors) {
      if (anchor.label !== null && services.foldCase(anchor.label).includes(needle)) {
        results.push({ kind: "anchor", page, anchor });
      }
    }
  }
  return results;
}

export function resolveSettingsDeepLink(
  registry: SettingsRegistrySnapshot,
  link: SettingsDeepLink,
): ResolvedSettingsDeepLink {
  const page = registry.pages.find((candidate) => candidate.id === link.pageId);
  if (page === undefined) {
    throw new SettingsDeepLinkResolutionError(`unknown settings page ${link.pageId}`, link);
  }
  if (link.anchorId === undefined) {
    return { page, anchor: undefined };
  }
  const anchor = page.anchors.find((candidate) => candidate.id === link.anchorId);
  if (anchor === undefined) {
    throw new SettingsDeepLinkResolutionError(
      `unknown settings anchor ${link.anchorId} on ${link.pageId}`,
      link,
    );
  }
  return { page, anchor };
}
