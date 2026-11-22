//! Optional config-operation settings registration conformance.

use longhorn_core::SettingsCapabilityId;
use longhorn_settings::{
    SettingsLimits, SettingsNavigationPresentation, SettingsPagePresentation,
    SettingsRegistryBuilder, SettingsRegistryGeneration,
};
use longhorn_settings_config::{
    BACKUP_INVENTORY_CAPABILITY_ID, BACKUP_SETTINGS_PAGE_ID, ConfigOperationsSettingsPresentation,
    RESTORE_INSPECTION_CAPABILITY_ID, RESTORE_SETTINGS_PAGE_ID, STORAGE_DIAGNOSTICS_CAPABILITY_ID,
    STORAGE_SETTINGS_PAGE_ID, register_config_operations_settings,
};

#[test]
fn pages_are_admitted_only_by_their_base_capabilities() {
    let registry = registry([
        STORAGE_DIAGNOSTICS_CAPABILITY_ID,
        BACKUP_INVENTORY_CAPABILITY_ID,
        RESTORE_INSPECTION_CAPABILITY_ID,
    ]);
    let pages = registry
        .pages()
        .map(|page| page.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        pages,
        [
            STORAGE_SETTINGS_PAGE_ID,
            BACKUP_SETTINGS_PAGE_ID,
            RESTORE_SETTINGS_PAGE_ID
        ]
    );
    assert_eq!(registry.modules().next().unwrap().label, "Stockage");
    assert_eq!(registry.sections().next().unwrap().label, "Données");
    assert_eq!(
        registry
            .pages()
            .map(|page| page.label.as_str())
            .collect::<Vec<_>>(),
        ["Stockage", "Sauvegardes", "Restauration"]
    );
    assert!(
        registry
            .pages()
            .all(|page| page.readable_scope_ids.is_empty()
                && page.writable_apply_unit_ids.is_empty())
    );
}

#[test]
fn absent_capabilities_leave_no_operational_page_or_section() {
    let registry = registry([]);
    assert_eq!(registry.pages().count(), 0);
    assert_eq!(registry.sections().count(), 0);
}

#[test]
fn storage_backup_and_restore_admit_independently() {
    let storage = registry([STORAGE_DIAGNOSTICS_CAPABILITY_ID]);
    assert_eq!(
        storage
            .pages()
            .map(|page| page.id.as_str())
            .collect::<Vec<_>>(),
        [STORAGE_SETTINGS_PAGE_ID]
    );
    let backup = registry([BACKUP_INVENTORY_CAPABILITY_ID]);
    assert_eq!(
        backup
            .pages()
            .map(|page| page.id.as_str())
            .collect::<Vec<_>>(),
        [BACKUP_SETTINGS_PAGE_ID]
    );
    let restore = registry([RESTORE_INSPECTION_CAPABILITY_ID]);
    assert_eq!(
        restore
            .pages()
            .map(|page| page.id.as_str())
            .collect::<Vec<_>>(),
        [RESTORE_SETTINGS_PAGE_ID]
    );
}

fn registry<const N: usize>(capabilities: [&str; N]) -> longhorn_settings::SettingsRegistry {
    let mut builder = SettingsRegistryBuilder::new(
        SettingsRegistryGeneration::new(1),
        SettingsLimits::default(),
    );
    register_config_operations_settings(
        &mut builder,
        ConfigOperationsSettingsPresentation {
            module: SettingsNavigationPresentation {
                label: "Stockage".into(),
                order: 700,
            },
            section: SettingsNavigationPresentation {
                label: "Données".into(),
                order: 0,
            },
            storage: presentation("Stockage", 0),
            backup: presentation("Sauvegardes", 10),
            restore: presentation("Restauration", 20),
        },
    )
    .unwrap();
    builder
        .seal(capabilities.map(|value| SettingsCapabilityId::new(value).unwrap()))
        .unwrap()
}

fn presentation(label: &str, order: i32) -> SettingsPagePresentation {
    SettingsPagePresentation {
        label: label.into(),
        keywords: Vec::new(),
        order,
    }
}
