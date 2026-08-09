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

use longhorn_settings::{
    SettingsModuleDefinition, SettingsPageDefinition, SettingsRegistry, SettingsSectionDefinition,
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

#[cfg(test)]
mod tests {
    use super::*;
    use longhorn_core::{SettingsModuleId, SettingsPageId, SettingsRendererId, SettingsSectionId};
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
}
