//! Optional capability-gated command keybinding settings registration.

use longhorn_core::{
    SettingsCapabilityId, SettingsModuleId, SettingsPageId, SettingsRendererId, SettingsSectionId,
};
use longhorn_settings::{
    SettingsCapabilityDefinition, SettingsModuleDefinition, SettingsPageDefinition,
    SettingsPageFeatures, SettingsRegistryBuilder, SettingsRegistryError,
    SettingsRendererDefinition, SettingsSectionDefinition,
};

/// Optional command settings module.
pub const COMMAND_SETTINGS_MODULE_ID: &str = "longhorn:command-settings";
/// Command settings navigation section.
pub const COMMAND_SETTINGS_SECTION_ID: &str = "longhorn:commands";
/// Shared keybinding page.
pub const KEYBINDING_SETTINGS_PAGE_ID: &str = "longhorn:keybindings";
/// Consumer-resolved public Poodle keybinding renderer.
pub const KEYBINDING_SETTINGS_RENDERER_ID: &str = "longhorn:commands.keybindings";
/// Sealed command catalogue composition capability.
pub const COMMAND_CATALOGUE_CAPABILITY_ID: &str = "longhorn:commands.catalogue";
/// Writable authoritative keymap composition capability.
pub const WRITABLE_KEYMAP_CAPABILITY_ID: &str = "longhorn:commands.writable-keymap";

/// Registers the optional keybinding page into an unsealed settings registry.
///
/// Registry seal admits the page only when both command catalogue and
/// writable-keymap capabilities are composed. The page declares no settings
/// scope or apply unit: mutation remains under the injected command keymap
/// authority.
pub fn register_command_settings(
    builder: &mut SettingsRegistryBuilder,
) -> Result<(), SettingsRegistryError> {
    let module_id = module_id();
    let section_id = section_id();
    builder.register_module(SettingsModuleDefinition {
        id: module_id.clone(),
        label: "Commands".into(),
        order: 600,
    })?;
    builder.register_section(SettingsSectionDefinition {
        id: section_id.clone(),
        module_id: module_id.clone(),
        label: "Commands".into(),
        order: 0,
    })?;
    for capability in [
        COMMAND_CATALOGUE_CAPABILITY_ID,
        WRITABLE_KEYMAP_CAPABILITY_ID,
    ] {
        builder.register_capability(SettingsCapabilityDefinition {
            id: capability_id(capability),
            module_id: module_id.clone(),
        })?;
    }
    builder.register_renderer(SettingsRendererDefinition {
        id: renderer_id(),
        module_id: module_id.clone(),
    })?;
    builder.register_page(SettingsPageDefinition {
        id: page_id(),
        module_id,
        section_id,
        renderer_id: renderer_id(),
        label: "Keybindings".into(),
        keywords: vec![
            "commands".into(),
            "hotkeys".into(),
            "keyboard".into(),
            "shortcuts".into(),
        ],
        order: 0,
        anchors: vec![],
        required_capabilities: vec![
            capability_id(COMMAND_CATALOGUE_CAPABILITY_ID),
            capability_id(WRITABLE_KEYMAP_CAPABILITY_ID),
        ],
        readable_scope_ids: vec![],
        writable_apply_unit_ids: vec![],
        features: SettingsPageFeatures::default(),
    })
}

fn module_id() -> SettingsModuleId {
    SettingsModuleId::new(COMMAND_SETTINGS_MODULE_ID).expect("static module id must be valid")
}

fn section_id() -> SettingsSectionId {
    SettingsSectionId::new(COMMAND_SETTINGS_SECTION_ID).expect("static section id must be valid")
}

fn page_id() -> SettingsPageId {
    SettingsPageId::new(KEYBINDING_SETTINGS_PAGE_ID).expect("static page id must be valid")
}

fn renderer_id() -> SettingsRendererId {
    SettingsRendererId::new(KEYBINDING_SETTINGS_RENDERER_ID)
        .expect("static renderer id must be valid")
}

fn capability_id(value: &str) -> SettingsCapabilityId {
    SettingsCapabilityId::new(value).expect("static capability id must be valid")
}
