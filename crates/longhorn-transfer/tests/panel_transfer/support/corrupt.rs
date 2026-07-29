use std::fs;

use longhorn_layout_config::PersistedLayoutDocument;
use serde_json::json;

use super::{Fixture, TestDomain, document};

pub fn write_instance_policy_violation(fixture: &Fixture, domain: &TestDomain) {
    let mut value = serde_json::to_value(PersistedLayoutDocument::new(
        domain.registry_digest().clone(),
        document(),
    ))
    .unwrap();
    value["document"]["panel_instances"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "panel:fixed:two",
            "definition_id": "panel:fixed",
        }));
    let containers = value["document"]["containers"].as_array_mut().unwrap();
    let target = containers
        .iter_mut()
        .find(|container| container["id"] == "container:target")
        .unwrap();
    let side = target["regions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|region| region["region_id"] == "region:side")
        .unwrap();
    side["panel_instance_ids"] = json!(["panel:fixed:two"]);
    side["active_panel_instance_id"] = json!("panel:fixed:two");

    let bytes = serde_json::to_vec_pretty(&json!({
        "domain": domain.descriptor().id(),
        "schemaVersion": domain.descriptor().schema_version(),
        "value": value,
    }))
    .unwrap();
    let path = fixture.path(domain);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
