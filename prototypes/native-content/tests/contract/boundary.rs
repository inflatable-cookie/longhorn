use std::{fs, path::Path};

use longhorn_native_content_prototype::{
    AttachGeneration, NativeContentFailureCode, NativeContentIslandId, NativeContentKindId,
    NativeContentRevision,
};

#[test]
fn ids_and_counters_are_bounded_and_never_wrap() {
    assert!(NativeContentIslandId::new("island:browser-1").is_ok());
    assert!(NativeContentKindId::new("kind:plugin.editor").is_ok());
    assert!(NativeContentIslandId::new("").is_err());
    assert!(NativeContentIslandId::new("Uppercase").is_err());
    assert!(NativeContentFailureCode::new("x".repeat(129)).is_err());
    assert!(NativeContentRevision::new(u64::MAX).checked_next().is_err());
    assert!(AttachGeneration::new(u64::MAX).checked_next().is_err());
}

#[test]
fn serialized_coordination_evidence_has_no_generic_product_payload() {
    let coordinator = super::support::coordinator(
        longhorn_native_content_prototype::NativeContentMechanism::ChildView,
    );
    let json = serde_json::to_string(&coordinator.plan().unwrap()).unwrap();
    for forbidden in [
        "payload",
        "url",
        "plugin",
        "midi",
        "camera",
        "gizmo",
        "raw_handle",
    ] {
        assert!(
            !json.contains(forbidden),
            "found forbidden token {forbidden}"
        );
    }
}

#[test]
fn private_manifest_and_source_keep_optional_host_edges_absent() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("publish = false"));
    assert!(manifest.contains("[workspace]"));
    for forbidden in ["tauri =", "wgpu", "poodle", "svelte", "vst3", "clap"] {
        assert!(
            !manifest.to_ascii_lowercase().contains(forbidden),
            "manifest contains {forbidden}"
        );
    }

    let mut source = String::new();
    collect_rs(&root.join("src"), &mut source);
    for forbidden in [
        "tauri::",
        "wgpu::",
        "poodle",
        "svelte",
        "vst3",
        "rawwindowhandle",
        "nsview",
    ] {
        assert!(
            !source.to_ascii_lowercase().contains(forbidden),
            "source contains {forbidden}"
        );
    }
}

fn collect_rs(path: &Path, output: &mut String) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push_str(&fs::read_to_string(path).unwrap());
        }
    }
}
