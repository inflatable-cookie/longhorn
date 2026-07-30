use longhorn_core::{
    SettingsCapabilityId, SettingsModuleId, SettingsPageId, SettingsRendererId, SettingsSectionId,
};
use longhorn_settings::{
    SettingsCapabilityDefinition, SettingsModuleDefinition, SettingsPageDefinition,
    SettingsPageFeatures, SettingsRegistryBuilder, SettingsRegistryError,
    SettingsRendererDefinition, SettingsSectionDefinition,
};

/// Shared settings module id for optional config operations.
pub const CONFIG_OPERATIONS_MODULE_ID: &str = "longhorn:config-operations";
/// Shared navigation section id.
pub const CONFIG_OPERATIONS_SECTION_ID: &str = "longhorn:storage-and-backup";
/// Storage diagnostics settings page id.
pub const STORAGE_SETTINGS_PAGE_ID: &str = "longhorn:storage";
/// Backup operations settings page id.
pub const BACKUP_SETTINGS_PAGE_ID: &str = "longhorn:backup";
/// Storage page renderer resolver id.
pub const STORAGE_SETTINGS_RENDERER_ID: &str = "longhorn:config.storage";
/// Backup page renderer resolver id.
pub const BACKUP_SETTINGS_RENDERER_ID: &str = "longhorn:config.backup";
/// Restore and recovery settings page id.
pub const RESTORE_SETTINGS_PAGE_ID: &str = "longhorn:restore";
/// Restore page renderer resolver id.
pub const RESTORE_SETTINGS_RENDERER_ID: &str = "longhorn:config.restore";
/// Storage diagnostics admission capability.
pub const STORAGE_DIAGNOSTICS_CAPABILITY_ID: &str = "longhorn:config.storage-diagnostics";
/// Storage transition operation capability.
pub const STORAGE_TRANSITION_CAPABILITY_ID: &str = "longhorn:config.storage-transition";
/// Backup inventory admission capability.
pub const BACKUP_INVENTORY_CAPABILITY_ID: &str = "longhorn:config.backup-inventory";
/// Backup create operation capability.
pub const BACKUP_CREATE_CAPABILITY_ID: &str = "longhorn:config.backup-create";
/// Backup export operation capability.
pub const BACKUP_EXPORT_CAPABILITY_ID: &str = "longhorn:config.backup-export";
/// Backup retention operation capability.
pub const BACKUP_RETENTION_CAPABILITY_ID: &str = "longhorn:config.backup-retention";
/// Restore inspection admission capability.
pub const RESTORE_INSPECTION_CAPABILITY_ID: &str = "longhorn:config.restore-inspection";
/// Ordinary restore execution capability.
pub const RESTORE_EXECUTION_CAPABILITY_ID: &str = "longhorn:config.restore-execution";
/// Custom-adapter restore capability.
pub const RESTORE_ADAPTER_EXECUTION_CAPABILITY_ID: &str =
    "longhorn:config.restore-adapter-execution";
/// Restore recovery capability.
pub const RESTORE_RECOVERY_CAPABILITY_ID: &str = "longhorn:config.restore-recovery";

/// Registers optional storage and backup pages into an unsealed registry.
///
/// The storage page is admitted only when the host composes storage
/// diagnostics. The backup page is admitted only for backup inventory.
/// Mutation capabilities affect page actions, not admission. These operational
/// pages intentionally declare no ordinary settings scopes or apply units.
pub fn register_config_operations_settings(
    builder: &mut SettingsRegistryBuilder,
) -> Result<(), SettingsRegistryError> {
    let module_id = module_id();
    let section_id = section_id();
    builder.register_module(SettingsModuleDefinition {
        id: module_id.clone(),
        label: "Storage".into(),
        order: 700,
    })?;
    builder.register_section(SettingsSectionDefinition {
        id: section_id.clone(),
        module_id: module_id.clone(),
        label: "Storage & Backups".into(),
        order: 0,
    })?;
    for capability in [
        STORAGE_DIAGNOSTICS_CAPABILITY_ID,
        STORAGE_TRANSITION_CAPABILITY_ID,
        BACKUP_INVENTORY_CAPABILITY_ID,
        BACKUP_CREATE_CAPABILITY_ID,
        BACKUP_EXPORT_CAPABILITY_ID,
        BACKUP_RETENTION_CAPABILITY_ID,
        RESTORE_INSPECTION_CAPABILITY_ID,
        RESTORE_EXECUTION_CAPABILITY_ID,
        RESTORE_ADAPTER_EXECUTION_CAPABILITY_ID,
        RESTORE_RECOVERY_CAPABILITY_ID,
    ] {
        builder.register_capability(SettingsCapabilityDefinition {
            id: capability_id(capability),
            module_id: module_id.clone(),
        })?;
    }
    builder.register_renderer(SettingsRendererDefinition {
        id: renderer_id(STORAGE_SETTINGS_RENDERER_ID),
        module_id: module_id.clone(),
    })?;
    builder.register_renderer(SettingsRendererDefinition {
        id: renderer_id(BACKUP_SETTINGS_RENDERER_ID),
        module_id: module_id.clone(),
    })?;
    builder.register_renderer(SettingsRendererDefinition {
        id: renderer_id(RESTORE_SETTINGS_RENDERER_ID),
        module_id: module_id.clone(),
    })?;
    builder.register_page(SettingsPageDefinition {
        id: page_id(STORAGE_SETTINGS_PAGE_ID),
        module_id: module_id.clone(),
        section_id: section_id.clone(),
        renderer_id: renderer_id(STORAGE_SETTINGS_RENDERER_ID),
        label: "Storage".into(),
        keywords: vec![
            "cache".into(),
            "configuration".into(),
            "files".into(),
            "profile".into(),
        ],
        order: 0,
        anchors: vec![],
        required_capabilities: vec![capability_id(STORAGE_DIAGNOSTICS_CAPABILITY_ID)],
        readable_scope_ids: vec![],
        writable_apply_unit_ids: vec![],
        features: SettingsPageFeatures {
            confirmation: true,
            ..SettingsPageFeatures::default()
        },
    })?;
    builder.register_page(SettingsPageDefinition {
        id: page_id(BACKUP_SETTINGS_PAGE_ID),
        module_id: module_id.clone(),
        section_id: section_id.clone(),
        renderer_id: renderer_id(BACKUP_SETTINGS_RENDERER_ID),
        label: "Backups".into(),
        keywords: vec![
            "archive".into(),
            "backup".into(),
            "encryption".into(),
            "export".into(),
            "retention".into(),
        ],
        order: 10,
        anchors: vec![],
        required_capabilities: vec![capability_id(BACKUP_INVENTORY_CAPABILITY_ID)],
        readable_scope_ids: vec![],
        writable_apply_unit_ids: vec![],
        features: SettingsPageFeatures {
            backup: true,
            confirmation: true,
            ..SettingsPageFeatures::default()
        },
    })?;
    builder.register_page(SettingsPageDefinition {
        id: page_id(RESTORE_SETTINGS_PAGE_ID),
        module_id,
        section_id,
        renderer_id: renderer_id(RESTORE_SETTINGS_RENDERER_ID),
        label: "Restore & Recovery".into(),
        keywords: vec![
            "archive".into(),
            "conflict".into(),
            "recovery".into(),
            "restore".into(),
            "rollback".into(),
        ],
        order: 20,
        anchors: vec![],
        required_capabilities: vec![capability_id(RESTORE_INSPECTION_CAPABILITY_ID)],
        readable_scope_ids: vec![],
        writable_apply_unit_ids: vec![],
        features: SettingsPageFeatures {
            restore: true,
            confirmation: true,
            ..SettingsPageFeatures::default()
        },
    })
}

fn module_id() -> SettingsModuleId {
    SettingsModuleId::new(CONFIG_OPERATIONS_MODULE_ID).expect("static module id must be valid")
}

fn section_id() -> SettingsSectionId {
    SettingsSectionId::new(CONFIG_OPERATIONS_SECTION_ID).expect("static section id must be valid")
}

fn page_id(value: &str) -> SettingsPageId {
    SettingsPageId::new(value).expect("static page id must be valid")
}

fn renderer_id(value: &str) -> SettingsRendererId {
    SettingsRendererId::new(value).expect("static renderer id must be valid")
}

fn capability_id(value: &str) -> SettingsCapabilityId {
    SettingsCapabilityId::new(value).expect("static capability id must be valid")
}
