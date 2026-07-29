use std::{error::Error, io};

use longhorn_core::{LayoutRevision, RegionId};
use longhorn_layout::{
    LAYOUT_PROTOCOL_VERSION, LayoutDefinitionRegistry, LayoutDocument, LayoutLimits,
    LayoutMutationCommand, LayoutMutationEngine, LayoutMutationReceipt, LayoutMutationRejection,
    LayoutMutationRejectionCode, LayoutMutationRequest, LayoutSchemaDefinition, PanelDefinition,
    RegionVisibility, project_region_visibility,
};
use serde::Serialize;

mod shape;

use shape::*;

#[derive(Serialize)]
struct ConformanceFixture {
    protocol_version: u32,
    name: &'static str,
    host_binding: HostBinding,
    resolved_default_region: RegionId,
    definitions: FixtureDefinitions,
    initial_document: LayoutDocument,
    steps: Vec<ConformanceStep>,
    singleton_policy: SingletonPolicyFixture,
    ordinary_visibility: Vec<RegionVisibility>,
    transient_visibility: Vec<RegionVisibility>,
    stale_rejection: LayoutMutationRejection,
    invalid_rejection: LayoutMutationRejection,
    expected_snapshot: LayoutDocument,
}

#[derive(Serialize)]
struct FixtureDefinitions {
    limits: LayoutLimits,
    schema: LayoutSchemaDefinition,
    panels: Vec<PanelDefinition>,
}

#[derive(Serialize)]
struct ConformanceStep {
    request: LayoutMutationRequest,
    receipt: LayoutMutationReceipt,
}

#[derive(Serialize)]
struct SingletonPolicyFixture {
    first_receipt: LayoutMutationReceipt,
    second_rejection: LayoutMutationRejection,
}

pub fn render() -> Result<Vec<(&'static str, String)>, Box<dyn Error>> {
    [loophole_spec(), nucleus_spec()]
        .into_iter()
        .map(|(path, spec)| {
            let fixture = build_fixture(spec)?;
            let mut contents = serde_json::to_string_pretty(&fixture)?;
            contents.push('\n');
            Ok((path, contents))
        })
        .collect()
}

fn build_fixture(spec: ShapeSpec) -> Result<ConformanceFixture, Box<dyn Error>> {
    let limits = limits();
    let schema = schema(&spec);
    let panels = panels(&spec);
    let registry = LayoutDefinitionRegistry::new(limits, [schema.clone()], panels.iter().cloned())?;
    let resolved_default_region = registry
        .default_region(
            &schema_id(spec.schema_id),
            &definition_id("panel:workspace-tool"),
        )?
        .ok_or_else(|| io::Error::other("workspace tool has no default region"))?;
    let initial_document = initial_document(&spec);
    let steps = run_shared_sequence(&spec, &registry, &initial_document)?;
    let expected_snapshot = steps
        .last()
        .expect("shared conformance sequence is nonempty")
        .receipt
        .authoritative_document()
        .clone();
    let engine = LayoutMutationEngine::new(&registry);
    let singleton_policy = singleton_policy(&spec, &engine, &initial_document)?;
    let ordinary_visibility = project_region_visibility(
        &registry,
        &expected_snapshot,
        &container_id("container:primary"),
        None,
    )?;
    let transient_visibility = project_region_visibility(
        &registry,
        &expected_snapshot,
        &container_id("container:primary"),
        Some(&definition_id("panel:workspace-tool")),
    )?;
    let stale_rejection = expect_rejection(
        &engine,
        &expected_snapshot,
        LayoutMutationRequest::new(
            request_id(&format!("request:{}:stale", spec.name)),
            LayoutRevision::INITIAL,
            LayoutMutationCommand::ActivatePanel {
                panel_instance_id: instance_id("instance:b"),
            },
        ),
        LayoutMutationRejectionCode::StaleRevision,
    )?;
    let invalid_rejection = expect_rejection(
        &engine,
        &expected_snapshot,
        LayoutMutationRequest::new(
            request_id(&format!("request:{}:unchanged-move", spec.name)),
            expected_snapshot.revision(),
            LayoutMutationCommand::MovePanel {
                panel_instance_id: instance_id("instance:b"),
                target_container_id: container_id("container:primary"),
                target_region_id: region_id(spec.target_region),
                insertion_index: 0,
            },
        ),
        LayoutMutationRejectionCode::MoveTargetUnchanged,
    )?;

    Ok(ConformanceFixture {
        protocol_version: LAYOUT_PROTOCOL_VERSION,
        name: spec.name,
        host_binding: spec.host_binding,
        resolved_default_region,
        definitions: FixtureDefinitions {
            limits,
            schema,
            panels,
        },
        initial_document,
        steps,
        singleton_policy,
        ordinary_visibility,
        transient_visibility,
        stale_rejection,
        invalid_rejection,
        expected_snapshot,
    })
}

fn run_shared_sequence(
    spec: &ShapeSpec,
    registry: &LayoutDefinitionRegistry,
    initial: &LayoutDocument,
) -> Result<Vec<ConformanceStep>, LayoutMutationRejection> {
    let engine = LayoutMutationEngine::new(registry);
    let mut current = initial.clone();
    let commands = [
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:a"),
            panel_definition_id: definition_id("panel:workspace-tool"),
            container_id: container_id("container:primary"),
            region_id: region_id(spec.source_region),
            insertion_index: 0,
        },
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:b"),
            panel_definition_id: definition_id("panel:workspace-tool"),
            container_id: container_id("container:primary"),
            region_id: region_id(spec.source_region),
            insertion_index: 1,
        },
        LayoutMutationCommand::ActivatePanel {
            panel_instance_id: instance_id("instance:a"),
        },
        LayoutMutationCommand::ReorderRegion {
            container_id: container_id("container:primary"),
            region_id: region_id(spec.source_region),
            panel_instance_ids: vec![instance_id("instance:b"), instance_id("instance:a")],
        },
        LayoutMutationCommand::MovePanel {
            panel_instance_id: instance_id("instance:b"),
            target_container_id: container_id("container:primary"),
            target_region_id: region_id(spec.target_region),
            insertion_index: 0,
        },
        LayoutMutationCommand::SetSizingSlot {
            container_id: container_id("container:primary"),
            sizing_slot_id: slot_id(spec.sizing_slots[0]),
            ratio: ratio(300_000),
        },
        LayoutMutationCommand::SetRegionCollapsed {
            container_id: container_id("container:primary"),
            region_id: region_id(spec.target_region),
            collapsed: true,
        },
        LayoutMutationCommand::ClosePanel {
            panel_instance_id: instance_id("instance:a"),
        },
    ];
    let names = [
        "create-a",
        "create-b",
        "activate-a",
        "reorder",
        "move-b",
        "size",
        "collapse",
        "close-a",
    ];
    let mut steps = Vec::with_capacity(commands.len());
    for (name, command) in names.into_iter().zip(commands) {
        let request = LayoutMutationRequest::new(
            request_id(&format!("request:{}:{name}", spec.name)),
            current.revision(),
            command,
        );
        let receipt = engine.apply(&current, &request)?;
        current = receipt.authoritative_document().clone();
        steps.push(ConformanceStep { request, receipt });
    }
    Ok(steps)
}

fn singleton_policy(
    spec: &ShapeSpec,
    engine: &LayoutMutationEngine<'_>,
    initial: &LayoutDocument,
) -> Result<SingletonPolicyFixture, Box<dyn Error>> {
    let first_request = LayoutMutationRequest::new(
        request_id(&format!("request:{}:singleton-a", spec.name)),
        initial.revision(),
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:singleton-a"),
            panel_definition_id: definition_id(spec.singleton_definition),
            container_id: container_id("container:primary"),
            region_id: region_id(spec.singleton_region),
            insertion_index: 0,
        },
    );
    let first_receipt = engine.apply(initial, &first_request)?;
    let second_request = LayoutMutationRequest::new(
        request_id(&format!("request:{}:singleton-b", spec.name)),
        first_receipt.committed_revision(),
        LayoutMutationCommand::CreatePanel {
            panel_instance_id: instance_id("instance:singleton-b"),
            panel_definition_id: definition_id(spec.singleton_definition),
            container_id: container_id("container:primary"),
            region_id: region_id(spec.singleton_region),
            insertion_index: 1,
        },
    );
    let second_rejection = expect_rejection(
        engine,
        first_receipt.authoritative_document(),
        second_request,
        LayoutMutationRejectionCode::InstancePolicyExceeded,
    )?;
    Ok(SingletonPolicyFixture {
        first_receipt,
        second_rejection,
    })
}

fn expect_rejection(
    engine: &LayoutMutationEngine<'_>,
    document: &LayoutDocument,
    request: LayoutMutationRequest,
    expected: LayoutMutationRejectionCode,
) -> Result<LayoutMutationRejection, Box<dyn Error>> {
    match engine.apply(document, &request) {
        Err(rejection) if rejection.code() == expected => Ok(rejection),
        Err(rejection) => Err(io::Error::other(format!(
            "expected {expected:?}; received {:?}",
            rejection.code()
        ))
        .into()),
        Ok(_) => Err(io::Error::other(format!("expected {expected:?} rejection")).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_shapes_complete_the_same_public_matrix() {
        let fixtures = [loophole_spec(), nucleus_spec()]
            .into_iter()
            .map(|(_, spec)| build_fixture(spec).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(fixtures[0].definitions.schema.regions().len(), 8);
        assert_eq!(fixtures[0].definitions.schema.sizing_slots().len(), 3);
        assert_eq!(fixtures[1].definitions.schema.regions().len(), 5);
        assert_eq!(fixtures[1].definitions.schema.sizing_slots().len(), 4);
        for fixture in fixtures {
            assert_eq!(fixture.steps.len(), 8);
            assert_eq!(fixture.expected_snapshot.revision(), LayoutRevision::new(8));
            assert_eq!(
                fixture.singleton_policy.second_rejection.code(),
                LayoutMutationRejectionCode::InstancePolicyExceeded
            );
            assert_eq!(
                fixture.stale_rejection.code(),
                LayoutMutationRejectionCode::StaleRevision
            );
            assert_eq!(
                fixture.invalid_rejection.code(),
                LayoutMutationRejectionCode::MoveTargetUnchanged
            );
        }
    }
}
