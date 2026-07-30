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
  readonly sections: readonly SettingsNavigationSection[];
}

export interface SettingsNavigationSection {
  readonly section: SettingsSectionDefinition;
  readonly pages: readonly SettingsPageDefinition[];
}

export interface SettingsRegistryProjection {
  readonly generation: number;
  readonly modules: readonly SettingsNavigationModule[];
  readonly pages: readonly SettingsPageDefinition[];
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
  const pagesBySection = new Map<string, SettingsPageDefinition[]>();
  for (const page of registry.pages) {
    const pages = pagesBySection.get(page.sectionId) ?? [];
    pages.push(page);
    pagesBySection.set(page.sectionId, pages);
  }
  const sectionsByModule = new Map<string, SettingsNavigationSection[]>();
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

export function searchSettingsRegistry(
  registry: SettingsRegistrySnapshot,
  query: string,
): readonly SettingsSearchResult[] {
  const needle = normalize(query);
  if (needle.length === 0) {
    return [];
  }
  const results: SettingsSearchResult[] = [];
  for (const page of registry.pages) {
    const pageMatch = [page.label, ...page.keywords]
      .map(normalize)
      .some((value) => value.includes(needle));
    if (pageMatch) {
      results.push({ kind: "page", page, anchor: undefined });
    }
    for (const anchor of page.anchors) {
      if (
        anchor.label !== null &&
        normalize(anchor.label).includes(needle)
      ) {
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
    throw new SettingsDeepLinkResolutionError(
      `unknown settings page ${link.pageId}`,
      link,
    );
  }
  if (link.anchorId === undefined) {
    return { page, anchor: undefined };
  }
  const anchor = page.anchors.find(
    (candidate) => candidate.id === link.anchorId,
  );
  if (anchor === undefined) {
    throw new SettingsDeepLinkResolutionError(
      `unknown settings anchor ${link.anchorId} on ${link.pageId}`,
      link,
    );
  }
  return { page, anchor };
}

function normalize(value: string): string {
  return value.trim().toLocaleLowerCase("en-US");
}
