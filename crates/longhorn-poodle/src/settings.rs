//! Projects the sealed settings registry into Poodle navigation specs.
//!
//! The Svelte tier splits this across two directories. `settings/poodle/`
//! holds one 435-line shell; `settings/svelte/` holds 1,308 lines of session,
//! document and page runtime built on Svelte runes. Only the shell's
//! navigation derivation is a projection, and it is the part this module
//! takes.
//!
//! The grouping rule itself currently lives in `packages/longhorn`, in
//! `projectSettingsRegistry` — a pure function over Rust-generated types,
//! written in TypeScript, in the client tier rather than the projection tier.
//! It is a Longhorn rule about Longhorn's own registry, so it belongs in
//! Rust; the TypeScript is the port, not the source.

use longhorn_core::{HostServices, SettingsAnchorId, SettingsPageId};
use longhorn_settings::{
    SettingsAnchorDefinition, SettingsModuleDefinition, SettingsPageDefinition, SettingsRegistry,
    SettingsSectionDefinition,
};
use poodle_specs::{SidebarNavGroup, SidebarNavItem, SidebarNavSpec};

/// One section and the pages it owns, in sealed order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationSection<'a> {
    /// The section definition.
    pub section: &'a SettingsSectionDefinition,
    /// Pages registered against this section.
    pub pages: Vec<&'a SettingsPageDefinition>,
}

/// One module and the sections it owns, in sealed order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationModule<'a> {
    /// The module definition.
    pub module: &'a SettingsModuleDefinition,
    /// Sections registered against this module that have at least one page.
    pub sections: Vec<NavigationSection<'a>>,
}

/// Groups a sealed registry into the module/section/page tree navigation
/// needs.
///
/// Empty sections and empty modules are dropped, because a navigation entry
/// that leads nowhere is worse than an absent one. Ordering is not applied
/// here: the registry builder sorts by explicit `order` then by id at seal,
/// so the sealed arrays are already in navigation order and re-sorting would
/// be a second, divergeable statement of the same rule.
#[must_use]
pub fn navigation(registry: &SettingsRegistry) -> Vec<NavigationModule<'_>> {
    registry
        .modules()
        .filter_map(|module| {
            let sections: Vec<NavigationSection<'_>> = registry
                .sections()
                .filter(|section| section.module_id == module.id)
                .filter_map(|section| {
                    let pages: Vec<&SettingsPageDefinition> = registry
                        .pages()
                        .filter(|page| page.section_id == section.id)
                        .collect();
                    if pages.is_empty() {
                        None
                    } else {
                        Some(NavigationSection { section, pages })
                    }
                })
                .collect();

            if sections.is_empty() {
                None
            } else {
                Some(NavigationModule { module, sections })
            }
        })
        .collect()
}

/// Builds the settings sidebar from a sealed registry.
///
/// Section labels are prefixed with their module only when more than one
/// module survives grouping. With a single module the prefix is noise, and
/// with several it is the only thing telling two identically-named sections
/// apart. This mirrors `SettingsShell.svelte` exactly, deliberately: the two
/// backends should not disagree about what a section is called.
#[must_use]
pub fn sidebar_nav(registry: &SettingsRegistry, selected: Option<&str>) -> SidebarNavSpec {
    let modules = navigation(registry);
    let qualify = modules.len() > 1;

    let groups: Vec<SidebarNavGroup> = modules
        .iter()
        .flat_map(|entry| {
            entry.sections.iter().map(move |section| {
                let items: Vec<SidebarNavItem> = section
                    .pages
                    .iter()
                    .map(|page| SidebarNavItem::new(page.id.as_str(), page.label.clone()))
                    .collect();

                let label = if qualify {
                    format!("{} \u{b7} {}", entry.module.label, section.section.label)
                } else {
                    section.section.label.clone()
                };

                SidebarNavGroup::new(section.section.id.as_str(), items).with_label(label)
            })
        })
        .collect();

    let mut spec = SidebarNavSpec::new(groups).with_aria_label("Settings");
    if let Some(value) = selected {
        spec = spec.with_value(value);
    }
    spec
}

/// What one search hit matched on.
///
/// A page and one of its anchors can both match the same query, and they are
/// different destinations — a page hit opens the page, an anchor hit scrolls
/// to a target within it. Collapsing them would lose the deep link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchMatch {
    /// The page's own label or keywords matched.
    Page,
    /// One of the page's anchor labels matched.
    Anchor,
}

/// One settings search result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResult<'a> {
    /// What matched.
    pub kind: SearchMatch,
    /// The page to open.
    pub page: &'a SettingsPageDefinition,
    /// The anchor to scroll to, for an anchor match.
    pub anchor: Option<&'a SettingsAnchorDefinition>,
}

/// Searches a sealed registry for pages and anchors matching `query`.
///
/// Case folding comes from `services` rather than from `str::to_lowercase`.
/// The Svelte tier pins `en-US`; Rust's default is locale-free; the two
/// disagree on Turkish dotless i and on any locale the product later ships
/// in. Making it the host's answer is what keeps the backends from choosing
/// independently — see memo 022, D4.
///
/// An empty or whitespace-only query returns nothing rather than everything.
/// A search box that has not been typed into is not a request to list the
/// whole registry.
#[must_use]
pub fn search<'a>(
    registry: &'a SettingsRegistry,
    query: &str,
    services: &impl HostServices,
) -> Vec<SearchResult<'a>> {
    let needle = services.fold_case(query.trim());
    if needle.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();
    for page in registry.pages() {
        let label_or_keyword = std::iter::once(page.label.as_str())
            .chain(page.keywords.iter().map(String::as_str))
            .any(|value| services.fold_case(value).contains(&needle));
        if label_or_keyword {
            results.push(SearchResult {
                kind: SearchMatch::Page,
                page,
                anchor: None,
            });
        }

        for anchor in &page.anchors {
            let Some(label) = anchor.label.as_deref() else {
                continue;
            };
            if services.fold_case(label).contains(&needle) {
                results.push(SearchResult {
                    kind: SearchMatch::Anchor,
                    page,
                    anchor: Some(anchor),
                });
            }
        }
    }
    results
}

/// Why a deep link could not be resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeepLinkError {
    /// No page carries this id in the sealed registry.
    UnknownPage(SettingsPageId),
    /// The page exists and does not declare this anchor.
    UnknownAnchor {
        /// The page that was found.
        page_id: SettingsPageId,
        /// The anchor that was not.
        anchor_id: SettingsAnchorId,
    },
}

/// A resolved deep link.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedDeepLink<'a> {
    /// The page to open.
    pub page: &'a SettingsPageDefinition,
    /// The anchor to scroll to, when the link named one.
    pub anchor: Option<&'a SettingsAnchorDefinition>,
}

/// Resolves a page id, and optionally an anchor id, against a sealed registry.
///
/// An unknown anchor is an error rather than a silent fall back to the top of
/// the page. A deep link that half-works is worse than one that says it is
/// stale: the first sends someone to the wrong place believing it is right.
pub fn resolve_deep_link<'a>(
    registry: &'a SettingsRegistry,
    page_id: &SettingsPageId,
    anchor_id: Option<&SettingsAnchorId>,
) -> Result<ResolvedDeepLink<'a>, DeepLinkError> {
    let page = registry
        .page(page_id)
        .ok_or_else(|| DeepLinkError::UnknownPage(page_id.clone()))?;

    let Some(anchor_id) = anchor_id else {
        return Ok(ResolvedDeepLink { page, anchor: None });
    };

    let anchor = page
        .anchors
        .iter()
        .find(|candidate| &candidate.id == anchor_id)
        .ok_or_else(|| DeepLinkError::UnknownAnchor {
            page_id: page_id.clone(),
            anchor_id: anchor_id.clone(),
        })?;

    Ok(ResolvedDeepLink {
        page,
        anchor: Some(anchor),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use longhorn_core::{
        PlainHostServices, SettingsModuleId, SettingsRendererId, SettingsSectionId,
    };
    use longhorn_settings::{
        SettingsLimits, SettingsPageFeatures, SettingsRegistryBuilder, SettingsRegistryGeneration,
        SettingsRendererDefinition,
    };

    struct Fixture {
        builder: SettingsRegistryBuilder,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                builder: SettingsRegistryBuilder::new(
                    SettingsRegistryGeneration::new(1),
                    SettingsLimits::default(),
                ),
            }
        }

        fn module(mut self, id: &str, label: &str, order: i32) -> Self {
            let module_id = SettingsModuleId::new(id).expect("module id");
            self.builder
                .register_module(SettingsModuleDefinition {
                    id: module_id.clone(),
                    label: label.to_owned(),
                    order,
                })
                .expect("module");
            self.builder
                .register_renderer(SettingsRendererDefinition {
                    id: SettingsRendererId::new(format!("{id}:renderer")).expect("renderer id"),
                    module_id,
                })
                .expect("renderer");
            self
        }

        fn section(mut self, id: &str, module: &str, label: &str, order: i32) -> Self {
            self.builder
                .register_section(SettingsSectionDefinition {
                    id: SettingsSectionId::new(id).expect("section id"),
                    module_id: SettingsModuleId::new(module).expect("module id"),
                    label: label.to_owned(),
                    order,
                })
                .expect("section");
            self
        }

        fn page_with(
            mut self,
            id: &str,
            module: &str,
            section: &str,
            label: &str,
            keywords: &[&str],
            anchors: &[(&str, Option<&str>)],
        ) -> Self {
            self.builder
                .register_page(SettingsPageDefinition {
                    id: SettingsPageId::new(id).expect("page id"),
                    module_id: SettingsModuleId::new(module).expect("module id"),
                    section_id: SettingsSectionId::new(section).expect("section id"),
                    renderer_id: SettingsRendererId::new(format!("{module}:renderer"))
                        .expect("renderer id"),
                    label: label.to_owned(),
                    keywords: keywords.iter().map(|k| (*k).to_owned()).collect(),
                    order: 0,
                    anchors: anchors
                        .iter()
                        .enumerate()
                        .map(
                            |(index, (anchor_id, anchor_label))| SettingsAnchorDefinition {
                                id: SettingsAnchorId::new(*anchor_id).expect("anchor id"),
                                label: anchor_label.map(ToOwned::to_owned),
                                order: i32::try_from(index).expect("order"),
                            },
                        )
                        .collect(),
                    required_capabilities: Vec::new(),
                    readable_scope_ids: Vec::new(),
                    writable_apply_unit_ids: Vec::new(),
                    features: SettingsPageFeatures::default(),
                })
                .expect("page");
            self
        }

        fn page(mut self, id: &str, module: &str, section: &str, label: &str) -> Self {
            self.builder
                .register_page(SettingsPageDefinition {
                    id: SettingsPageId::new(id).expect("page id"),
                    module_id: SettingsModuleId::new(module).expect("module id"),
                    section_id: SettingsSectionId::new(section).expect("section id"),
                    renderer_id: SettingsRendererId::new(format!("{module}:renderer"))
                        .expect("renderer id"),
                    label: label.to_owned(),
                    keywords: Vec::new(),
                    order: 0,
                    anchors: Vec::new(),
                    required_capabilities: Vec::new(),
                    readable_scope_ids: Vec::new(),
                    writable_apply_unit_ids: Vec::new(),
                    features: SettingsPageFeatures::default(),
                })
                .expect("page");
            self
        }

        fn seal(self) -> SettingsRegistry {
            self.builder.seal(Vec::new()).expect("seal")
        }
    }

    fn one_module() -> SettingsRegistry {
        Fixture::new()
            .module("core", "Core", 0)
            .section("general", "core", "General", 0)
            .page("appearance", "core", "general", "Appearance")
            .seal()
    }

    #[test]
    fn a_section_with_no_pages_is_dropped() {
        // A navigation entry that leads nowhere is worse than an absent one.
        let registry = Fixture::new()
            .module("core", "Core", 0)
            .section("general", "core", "General", 0)
            .section("empty", "core", "Empty", 1)
            .page("appearance", "core", "general", "Appearance")
            .seal();

        let modules = navigation(&registry);
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].sections.len(), 1);
        assert_eq!(modules[0].sections[0].section.id.as_str(), "general");
    }

    #[test]
    fn a_module_whose_every_section_is_empty_is_dropped_too() {
        let registry = Fixture::new()
            .module("core", "Core", 0)
            .module("plugins", "Plugins", 1)
            .section("general", "core", "General", 0)
            .section("installed", "plugins", "Installed", 0)
            .page("appearance", "core", "general", "Appearance")
            .seal();

        let modules = navigation(&registry);
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].module.id.as_str(), "core");
    }

    #[test]
    fn a_single_module_leaves_section_labels_unqualified() {
        let spec = sidebar_nav(&one_module(), None);

        assert_eq!(spec.groups.len(), 1);
        assert_eq!(spec.groups[0].label.as_deref(), Some("General"));
    }

    #[test]
    fn several_modules_qualify_every_section_label() {
        // Two sections sharing a label is exactly the case the prefix exists
        // for; without it the sidebar would show "General" twice.
        let registry = Fixture::new()
            .module("core", "Core", 0)
            .module("plugins", "Plugins", 1)
            .section("core:general", "core", "General", 0)
            .section("plugins:general", "plugins", "General", 0)
            .page("appearance", "core", "core:general", "Appearance")
            .page("installed", "plugins", "plugins:general", "Installed")
            .seal();

        let spec = sidebar_nav(&registry, None);
        let labels: Vec<&str> = spec
            .groups
            .iter()
            .filter_map(|group| group.label.as_deref())
            .collect();

        assert_eq!(
            labels,
            vec!["Core \u{b7} General", "Plugins \u{b7} General"]
        );
    }

    #[test]
    fn the_selected_page_is_carried_onto_the_spec() {
        let spec = sidebar_nav(&one_module(), Some("appearance"));
        assert_eq!(spec.value.as_deref(), Some("appearance"));
    }
    fn searchable() -> SettingsRegistry {
        Fixture::new()
            .module("core", "Core", 0)
            .section("general", "core", "General", 0)
            .page_with(
                "appearance",
                "core",
                "general",
                "Appearance",
                &["theme", "colour"],
                &[
                    ("appearance:contrast", Some("Contrast")),
                    ("appearance:hidden", None),
                ],
            )
            .seal()
    }

    #[test]
    fn an_empty_query_returns_nothing_rather_than_everything() {
        // A search box nobody has typed into is not a request for the whole
        // registry.
        let registry = searchable();
        for query in ["", "   ", "\t"] {
            assert!(
                search(&registry, query, &PlainHostServices::default()).is_empty(),
                "{query:?}"
            );
        }
    }

    #[test]
    fn a_page_matches_on_its_label_and_on_its_keywords() {
        let registry = searchable();
        let services = PlainHostServices::default();

        for query in ["appear", "THEME", "colour"] {
            let hits = search(&registry, query, &services);
            assert_eq!(hits.len(), 1, "{query}");
            assert_eq!(hits[0].kind, SearchMatch::Page, "{query}");
            assert!(hits[0].anchor.is_none(), "{query}");
        }
    }

    #[test]
    fn an_anchor_match_is_a_separate_destination() {
        // Page and anchor hits open different places, so they stay distinct.
        let registry = searchable();
        let hits = search(&registry, "contrast", &PlainHostServices::default());

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, SearchMatch::Anchor);
        assert_eq!(
            hits[0].anchor.expect("anchor").id.as_str(),
            "appearance:contrast"
        );
    }

    #[test]
    fn an_unlabelled_anchor_never_matches() {
        // It has no text to match against, and matching it on its id would
        // expose an identifier as though it were a name.
        let registry = searchable();
        let hits = search(&registry, "hidden", &PlainHostServices::default());
        assert!(hits.is_empty());
    }

    #[test]
    fn case_folding_comes_from_the_host() {
        // The whole point of taking `HostServices`: a host with locale rules
        // supplies them and the projection does not guess. A host that folds
        // nothing makes search case-sensitive, which is a real answer some
        // host could want — and proves the projection is not folding behind
        // its back.
        struct FoldsNothing;

        impl HostServices for FoldsNothing {
            fn new_request_id(&self) -> String {
                "test".to_owned()
            }

            fn format_timestamp(&self, unix_seconds: i64) -> String {
                unix_seconds.to_string()
            }

            fn fold_case(&self, value: &str) -> String {
                value.to_owned()
            }
        }

        let registry = searchable();
        assert!(search(&registry, "appear", &FoldsNothing).is_empty());
        assert_eq!(search(&registry, "Appear", &FoldsNothing).len(), 1);
        // The default host folds, so the lowercase query matches there.
        assert_eq!(
            search(&registry, "appear", &PlainHostServices::default()).len(),
            1
        );
    }

    #[test]
    fn a_deep_link_to_a_page_alone_resolves() {
        let registry = searchable();
        let page = SettingsPageId::new("appearance").expect("page id");

        let resolved = resolve_deep_link(&registry, &page, None).expect("resolved");
        assert_eq!(resolved.page.id, page);
        assert!(resolved.anchor.is_none());
    }

    #[test]
    fn an_unknown_anchor_is_an_error_rather_than_the_top_of_the_page() {
        // A deep link that half-works sends someone to the wrong place
        // believing it is right.
        let registry = searchable();
        let page = SettingsPageId::new("appearance").expect("page id");
        let anchor = SettingsAnchorId::new("appearance:gone").expect("anchor id");

        let error = resolve_deep_link(&registry, &page, Some(&anchor)).expect_err("stale link");
        assert_eq!(
            error,
            DeepLinkError::UnknownAnchor {
                page_id: page,
                anchor_id: anchor
            }
        );
    }

    #[test]
    fn an_unknown_page_names_the_page_it_could_not_find() {
        let missing = SettingsPageId::new("nope").expect("page id");
        let error = resolve_deep_link(&searchable(), &missing, None).expect_err("unknown");
        assert_eq!(error, DeepLinkError::UnknownPage(missing));
    }
}
