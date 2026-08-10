use std::error::Error;

use longhorn_core::{
    LayoutContainerId, PanelDefinitionId, SurfaceId, SurfaceRequestId, SurfaceRevision, WindowId,
};
use longhorn_surfaces::{
    EmptyWindowPolicy, LayoutContainerInventory, ParticipatingWindow, SURFACE_PROTOCOL_VERSION,
    SurfaceChangedEvent, SurfaceDocument, SurfaceHostPreference, SurfaceLimits,
    SurfaceMutationCommand, SurfaceMutationEngine, SurfaceMutationReceipt,
    SurfaceMutationRejection, SurfaceMutationRequest, SurfaceMutationResponse, SurfacePresentation,
    SurfaceProtocolEpoch, SurfaceRecord, SurfaceSnapshot,
};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
struct GoldenFixture {
    protocol_version: u32,
    snapshots: Vec<SurfaceSnapshot>,
    commands: Vec<SurfaceMutationRequest>,
    receipts: Vec<SurfaceMutationReceipt>,
    errors: Vec<SurfaceMutationRejection>,
    responses: Vec<SurfaceMutationResponse>,
    events: Vec<SurfaceChangedEvent>,
    incompatibility: IncompatibilityFixture,
}

#[derive(Serialize)]
struct IncompatibilityFixture {
    future_protocol_version: u32,
    unknown_command: Value,
    unknown_outcome: Value,
    unknown_rejection_code: Value,
}

pub fn render(rejection_codes: &[String]) -> Result<String, Box<dyn Error>> {
    let source = document();
    let requests = requests(source.revision());
    let containers = LayoutContainerInventory::new(
        [
            "container:mix",
            "container:edit",
            "container:plugins",
            "container:new",
            "container:duplicate",
        ]
        .into_iter()
        .map(container_id),
    );
    let engine = SurfaceMutationEngine::new(
        SurfaceLimits::new(8, 4, 4, 64)?,
        &containers,
        EmptyWindowPolicy::Allow,
    );
    let receipts: Vec<_> = requests
        .iter()
        .map(|request| engine.apply(&source, request))
        .collect::<Result<_, _>>()?;
    let stale = SurfaceMutationRequest::new(
        request_id("request:stale"),
        SurfaceRevision::INITIAL,
        SurfaceMutationCommand::RenameSurface {
            surface_id: surface_id("surface:mix"),
            label: Some("Stale".to_owned()),
        },
    );
    let stale_rejection = engine
        .apply(&source, &stale)
        .expect_err("stale fixture request must be rejected");
    let errors = rejection_fixtures(&source, &stale_rejection, rejection_codes)?;
    let responses = receipts
        .iter()
        .cloned()
        .map(|receipt| SurfaceMutationResponse::Committed { receipt })
        .chain(
            errors
                .iter()
                .cloned()
                .map(|rejection| SurfaceMutationResponse::Rejected { rejection }),
        )
        .collect();
    let epoch = SurfaceProtocolEpoch::new(7);
    let snapshots =
        std::iter::once(SurfaceSnapshot::new(epoch, source))
            .chain(receipts.iter().map(|receipt| {
                SurfaceSnapshot::new(epoch, receipt.authoritative_document().clone())
            }))
            .collect();
    let events = receipts
        .iter()
        .map(|receipt| SurfaceChangedEvent::new(epoch, receipt.committed_revision()))
        .collect();
    let fixture = GoldenFixture {
        protocol_version: SURFACE_PROTOCOL_VERSION,
        snapshots,
        commands: requests,
        receipts,
        errors,
        responses,
        events,
        incompatibility: IncompatibilityFixture {
            future_protocol_version: SURFACE_PROTOCOL_VERSION + 1,
            unknown_command: json!({"kind": "future_surface_command"}),
            unknown_outcome: json!({"kind": "future_surface_outcome"}),
            unknown_rejection_code: json!("future_surface_rejection"),
        },
    };

    let mut rendered = serde_json::to_string_pretty(&fixture)?;
    rendered.push('\n');
    Ok(rendered)
}

fn rejection_fixtures(
    source: &SurfaceDocument,
    stale_rejection: &SurfaceMutationRejection,
    rejection_codes: &[String],
) -> Result<Vec<SurfaceMutationRejection>, serde_json::Error> {
    rejection_codes
        .iter()
        .enumerate()
        .map(|(index, code)| {
            if *code == "stale_revision" {
                return Ok(stale_rejection.clone());
            }
            serde_json::from_value(json!({
                "request_id": format!("request:error-{index:02}"),
                "current_revision": source.revision(),
                "code": code,
                "detail": format!("golden fixture for {code}"),
                "authoritative_document": source,
            }))
        })
        .collect()
}

fn requests(revision: SurfaceRevision) -> Vec<SurfaceMutationRequest> {
    vec![
        request(
            "request:create",
            revision,
            SurfaceMutationCommand::CreateSurface {
                surface_id: surface_id("surface:new"),
                layout_container_id: container_id("container:new"),
                label: Some("New".to_owned()),
                host_preferences: vec![host("window:main", 2), host("window:tools", 3)],
            },
        ),
        request(
            "request:duplicate",
            revision,
            SurfaceMutationCommand::DuplicateSurface {
                source_surface_id: surface_id("surface:mix"),
                surface_id: surface_id("surface:copy"),
                layout_container_id: container_id("container:duplicate"),
            },
        ),
        request(
            "request:rename",
            revision,
            SurfaceMutationCommand::RenameSurface {
                surface_id: surface_id("surface:mix"),
                label: Some("Mix room".to_owned()),
            },
        ),
        request(
            "request:focus",
            revision,
            SurfaceMutationCommand::SetSurfacePresentation {
                surface_id: surface_id("surface:plugins"),
                presentation: SurfacePresentation::FocusedPanel {
                    panel_definition_id: PanelDefinitionId::new("panel:plugin-manager")
                        .expect("panel definition id"),
                },
            },
        ),
        request(
            "request:activate",
            revision,
            SurfaceMutationCommand::ActivateSurface {
                window_id: window_id("window:main"),
                surface_id: surface_id("surface:mix"),
            },
        ),
        request(
            "request:reorder",
            revision,
            SurfaceMutationCommand::ReorderWindow {
                window_id: window_id("window:main"),
                surface_ids: vec![surface_id("surface:edit"), surface_id("surface:mix")],
            },
        ),
        request(
            "request:move",
            revision,
            SurfaceMutationCommand::MoveSurface {
                surface_id: surface_id("surface:mix"),
                target_window_id: window_id("window:tools"),
                insertion_index: 1,
            },
        ),
        request(
            "request:close",
            revision,
            SurfaceMutationCommand::CloseSurface {
                surface_id: surface_id("surface:plugins"),
            },
        ),
    ]
}

fn request(
    id: &str,
    revision: SurfaceRevision,
    command: SurfaceMutationCommand,
) -> SurfaceMutationRequest {
    SurfaceMutationRequest::new(request_id(id), revision, command)
}

fn document() -> SurfaceDocument {
    SurfaceDocument::new(
        SurfaceRevision::new(11),
        [
            surface(
                "surface:mix",
                "container:mix",
                Some("Mix"),
                [host("window:main", 0), host("window:tools", 1)],
            ),
            surface(
                "surface:edit",
                "container:edit",
                Some("Edit"),
                [host("window:main", 1), host("window:tools", 0)],
            ),
            surface(
                "surface:plugins",
                "container:plugins",
                None,
                [host("window:tools", 2)],
            ),
        ],
        [
            ParticipatingWindow::new(window_id("window:main"), Some(surface_id("surface:edit"))),
            ParticipatingWindow::new(
                window_id("window:tools"),
                Some(surface_id("surface:plugins")),
            ),
        ],
    )
}

fn surface(
    id: &str,
    container: &str,
    label: Option<&str>,
    preferences: impl IntoIterator<Item = SurfaceHostPreference>,
) -> SurfaceRecord {
    SurfaceRecord::new(
        surface_id(id),
        container_id(container),
        label.map(ToOwned::to_owned),
        preferences,
    )
}

fn host(window: &str, order: u32) -> SurfaceHostPreference {
    SurfaceHostPreference::new(window_id(window), order)
}

fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).expect("fixture Surface id is valid")
}

fn container_id(value: &str) -> LayoutContainerId {
    LayoutContainerId::new(value).expect("fixture container id is valid")
}

fn window_id(value: &str) -> WindowId {
    WindowId::new(value).expect("fixture window id is valid")
}

fn request_id(value: &str) -> SurfaceRequestId {
    SurfaceRequestId::new(value).expect("fixture request id is valid")
}
