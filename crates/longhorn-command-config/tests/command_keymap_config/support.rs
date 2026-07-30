use std::{fs, path::PathBuf, time::Duration};

use longhorn_command::{
    CommandArgumentSchema, CommandBindingDefinition, CommandContextDefinition, CommandDefinition,
    CommandKeyTrigger, CommandKeymapPreset, CommandLimits, CommandPhysicalCode,
    CommandPlatformScope, CommandRegistry, CommandRegistryBuilder, CommandRegistryGeneration,
    CommandTextInputPolicy, CommandTriggerModifiers, CommandVisibility, NoReservedCommandChords,
};
use longhorn_command_config::{
    CommandKeymapBackupPolicy, CommandKeymapPatch, CommandKeymapPreview,
    CommandKeymapPreviewResult, CommandKeymapService, NoCommandKeymapMigration,
    RegisteredCommandKeymapDomain,
};
use longhorn_config::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath,
    DomainLocation, DurabilityRequirement, MutationOptions, StorageClass, StorageRoots,
};
use longhorn_core::{
    CommandBindingId, CommandCategoryId, CommandContextId, CommandId, CommandRequestId,
    CommandRouteId, DomainId, SchemaVersion,
};
use serde_json::{Value, json};
use tempfile::TempDir;

pub type TestDomain =
    RegisteredCommandKeymapDomain<NoReservedCommandChords, NoCommandKeymapMigration>;
pub type TestService = CommandKeymapService<NoReservedCommandChords, NoCommandKeymapMigration>;

pub struct Fixture {
    _temp: TempDir,
    roots: StorageRoots,
    coordination: CoordinationAuthority,
}

impl Fixture {
    pub fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let data = root.join("data");
        for path in [
            root.join("config"),
            data.clone(),
            root.join("state"),
            root.join("cache"),
            root.join("runtime"),
            root.join("log"),
            root.join("backups"),
        ] {
            fs::create_dir_all(path).unwrap();
        }
        Self {
            roots: StorageRoots::new(
                root.join("config"),
                &data,
                root.join("state"),
                root.join("cache"),
                root.join("runtime"),
                root.join("log"),
                root.join("backups"),
            )
            .unwrap(),
            coordination: CoordinationAuthority::new(data).unwrap(),
            _temp: temp,
        }
    }

    pub fn store<D: ConfigDomain>(&self, domain: &D) -> ConfigStore {
        let mut store = ConfigStore::new(self.roots.clone(), self.coordination.clone());
        store.register(domain).unwrap();
        store
    }

    pub fn path<D: ConfigDomain>(&self, domain: &D) -> PathBuf {
        match self.roots.resolve(domain.descriptor()) {
            DomainLocation::File(file) => file.full_path().to_path_buf(),
            other => panic!("expected file domain, found {other:?}"),
        }
    }

    pub fn write<D: ConfigDomain>(&self, domain: &D, bytes: &[u8]) {
        let path = self.path(domain);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

pub fn id<T>(value: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    value.parse().unwrap()
}

pub fn descriptor(version: u32) -> DomainDescriptor {
    DomainDescriptor::new(
        DomainId::new("commands.keymap").unwrap(),
        SchemaVersion::new(version).unwrap(),
        StorageClass::UserConfig,
        Some(DomainFilePath::new("commands/keymap.json").unwrap()),
    )
    .unwrap()
}

pub fn registry(generation: u64) -> CommandRegistry {
    let mut builder = CommandRegistryBuilder::new(
        CommandRegistryGeneration::new(generation),
        CommandLimits::default(),
    );
    builder
        .register_context(CommandContextDefinition {
            id: id("global"),
            parent_id: None,
        })
        .unwrap();
    for value in ["app:open", "app:save"] {
        builder
            .register_command(CommandDefinition {
                id: id(value),
                label: value.to_owned(),
                description: None,
                category_path: vec![id::<CommandCategoryId>("general")],
                keywords: Vec::new(),
                icon: None,
                allowed_contexts: vec![id::<CommandContextId>("global")],
                required_capabilities: Vec::new(),
                visibility: CommandVisibility::ALL,
                text_input_policy: CommandTextInputPolicy::Blocked,
                route: id::<CommandRouteId>(&format!("route:{value}")),
                arguments: CommandArgumentSchema::None,
            })
            .unwrap();
    }
    builder.seal().unwrap()
}

pub fn trigger(code: &str) -> CommandKeyTrigger {
    CommandKeyTrigger {
        code: CommandPhysicalCode::new(code).unwrap(),
        modifiers: CommandTriggerModifiers {
            primary: true,
            ..CommandTriggerModifiers::default()
        },
    }
}

pub fn binding(id_value: &str, code: &str, command: &str) -> CommandBindingDefinition {
    CommandBindingDefinition {
        id: id(id_value),
        platform: CommandPlatformScope::Any,
        trigger: trigger(code),
        context_id: id("global"),
        command_id: id::<CommandId>(command),
        arguments: Value::Null,
    }
}

pub fn preset(
    id_value: &str,
    version: u32,
    bindings: Vec<CommandBindingDefinition>,
) -> CommandKeymapPreset {
    CommandKeymapPreset {
        id: id(id_value),
        version: SchemaVersion::new(version).unwrap(),
        bindings,
    }
}

pub fn domain() -> TestDomain {
    RegisteredCommandKeymapDomain::new(
        descriptor(1),
        registry(1),
        vec![preset(
            "app:default",
            1,
            vec![
                binding("base:open", "KeyO", "app:open"),
                binding("base:save", "KeyS", "app:save"),
            ],
        )],
        id("app:default"),
        NoReservedCommandChords,
        NoCommandKeymapMigration,
        CommandKeymapBackupPolicy::Include,
    )
    .unwrap()
}

pub fn empty_patch() -> CommandKeymapPatch {
    CommandKeymapPatch {
        active_preset_id: None,
        clear_overrides: false,
        remove_binding_ids: Vec::new(),
        upsert_overrides: Vec::new(),
    }
}

pub fn preview(
    service: &TestService,
    store: &ConfigStore,
    patch: CommandKeymapPatch,
) -> CommandKeymapPreviewResult {
    let loaded = service.load(store, Duration::from_secs(2)).unwrap();
    let longhorn_command_config::CommandKeymapLoadOutcome::Loaded { snapshot } = loaded else {
        panic!("expected loaded keymap");
    };
    service
        .preview(
            store,
            &CommandKeymapPreview {
                registry_generation: snapshot.registry_generation,
                keymap_revision: snapshot.state.revision,
                active_preset_id: snapshot.state.active_preset_id,
                active_preset_version: snapshot.active_preset_version,
                patch,
            },
            Duration::from_secs(2),
        )
        .unwrap()
}

pub fn options() -> MutationOptions {
    MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic)
}

pub fn request_id(value: &str) -> CommandRequestId {
    id(value)
}

pub fn binding_id(value: &str) -> CommandBindingId {
    id(value)
}

pub fn envelope(version: u32, value: Value) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "domain": "commands.keymap",
        "schemaVersion": version,
        "value": value,
    }))
    .unwrap()
}
