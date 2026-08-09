//! Draws Longhorn domain facts in a real GPUI window.
//!
//! The end-to-end proof Card 169's last acceptance criterion asks for. Every
//! stage is real — no fixture stands in for a layer:
//!
//! ```text
//! NotificationLedger / UpdateAvailability / Usability   Longhorn domains
//!   -> longhorn-poodle                                  projection
//!     -> poodle-render        Spec + Theme -> Node      pure component tier
//!       -> poodle-gpui-node-backend  Node -> AnyElement GPUI interpretation
//!         -> gpui                                       pixels
//! ```
//!
//! Run with `cargo run --bin render` from `prototypes/gpui-windowing`. The
//! window stays open until closed; nothing here asserts, because the claim
//! being proved is "this draws", and a person has to look.

use gpui::{
    App, AppContext, Application, Bounds, Context, IntoElement, ParentElement, Render, Styled,
    Window, WindowBounds, WindowOptions, div, px, size,
};
use longhorn_core::HostServices;
use longhorn_core::{NotificationAuthorityId, NotificationId, NotificationSourceId};
use longhorn_licence::{Timestamp, Usability};
use longhorn_notifications::{
    NotificationAdd, NotificationAuthorityEpoch, NotificationDraft, NotificationLedger,
    NotificationLedgerLimits, NotificationSeverity, NotificationSummary, NotificationTitle,
};
use longhorn_operation::{OperationOverallProgressProjection, OperationStateProjection};
use longhorn_poodle::{
    licence::usability_banner, operation, project_notification_stack, update::availability_banner,
};
use longhorn_update::{InstallManager, UpdateAvailability};
use poodle_adapter::ThemeProvider;
use poodle_gpui::GpuiThemeProvider;
use poodle_render::toast_stack::ToastStackHandlers;
use poodle_render::{banner, progress, status_indicator, toast_stack};
use semver::Version;

/// Builds a ledger holding one notification per severity.
///
/// A real ledger with real publications rather than hand-built records: the
/// severity collapse this projection reports is only interesting if the
/// severities came through the domain's own admission path.
fn notifications() -> NotificationLedger {
    let mut ledger = NotificationLedger::new(
        NotificationAuthorityId::new("notifications:render-proof").expect("authority"),
        NotificationAuthorityEpoch::new(1).expect("epoch"),
        NotificationLedgerLimits::new(500, 32 * 1_024 * 1_024).expect("limits"),
    );

    for (index, (severity, title, summary)) in [
        (
            NotificationSeverity::Success,
            "Backup complete",
            "Wrote 1.2 GB to the archive.",
        ),
        (
            NotificationSeverity::Warning,
            "Restore needs a migration",
            "Two domains are one schema behind.",
        ),
        (
            NotificationSeverity::Error,
            "Sync failed",
            "The remote refused the last three attempts.",
        ),
        (
            NotificationSeverity::Critical,
            "Storage is read-only",
            "The volume was remounted without write access.",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let draft = NotificationDraft::new(
            NotificationSourceId::new("render-proof").expect("source"),
            severity,
            NotificationTitle::new(title).expect("title"),
            NotificationSummary::new(summary).expect("summary"),
        );
        let add = NotificationAdd::new(
            ledger.authority().clone(),
            ledger.revision(),
            NotificationId::new(format!("render-proof:{index}")).expect("id"),
            draft,
        );
        // `add` rather than `publish_once`: publish-once is the idempotent
        // path and wants a producer token, which a fixture has no honest
        // value for.
        ledger.add(add).expect("publish");
    }

    ledger
}

/// The host facilities a real application would supply. Stubbed here with
/// fixed answers, because the claim being proved is that they reach the
/// surface, not that any particular formatting is right.
struct ProofHost;

impl HostServices for ProofHost {
    fn new_request_id(&self) -> String {
        "render-proof:1".to_owned()
    }

    fn format_timestamp(&self, _unix_seconds: i64) -> String {
        "9 August 2026".to_owned()
    }

    fn fold_case(&self, value: &str) -> String {
        value.to_lowercase()
    }
}

struct ProofRoot {
    theme: GpuiThemeProvider,
    ledger: NotificationLedger,
}

impl Render for ProofRoot {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        poodle_gpui_node_backend::reset_element_ids();
        let theme = &self.theme;

        let records: Vec<_> = self.ledger.records().cloned().collect();
        let stack = project_notification_stack(&records);

        // Licence: the one state that is loud, with dates supplied by the
        // caller because `Timestamp` has no human form of its own.
        let lapsed = usability_banner(
            &Usability::LeaseLapsed {
                at: Timestamp::from_unix_seconds(1_754_697_600),
            },
            &ProofHost,
        )
        .expect("a lapsed lease warrants a banner");

        // Update: the state that looks like a broken updater unless said.
        let managed = availability_banner(
            &UpdateAvailability::ManagedElsewhere {
                version: Version::parse("1.3.0").expect("version"),
                manager: InstallManager::HomebrewCask,
            },
            "soundcheck",
        )
        .expect("a managed install warrants a banner");

        let running = operation::status_indicator(OperationStateProjection::Running);
        let cancelling = operation::status_indicator(OperationStateProjection::Cancelling);
        let units = operation::progress(OperationOverallProgressProjection::Units {
            completed: 3.0,
            total: 7.0,
        });

        let canvas =
            poodle_gpui_node_backend::color(theme.resolve_color("color.background.canvas"));
        let text = poodle_gpui_node_backend::color(theme.resolve_color("color.text.primary"));

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(20.0))
            .p(px(24.0))
            .bg(canvas)
            .text_color(text)
            .child(poodle_gpui_node_backend::to_gpui(&banner::banner(
                &lapsed, theme,
            )))
            .child(poodle_gpui_node_backend::to_gpui(&banner::banner(
                &managed, theme,
            )))
            .child(
                div()
                    .flex()
                    .gap(px(16.0))
                    .child(poodle_gpui_node_backend::to_gpui(
                        &status_indicator::status_indicator(&running, theme),
                    ))
                    .child(poodle_gpui_node_backend::to_gpui(
                        &status_indicator::status_indicator(&cancelling, theme),
                    )),
            )
            .child(poodle_gpui_node_backend::to_gpui(&progress::progress(
                &units, theme,
            )))
            .child(poodle_gpui_node_backend::to_gpui(
                &toast_stack::toast_stack(&stack, theme, ToastStackHandlers::default()),
            ))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(680.0), px(720.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| ProofRoot {
                    theme: GpuiThemeProvider::new(),
                    ledger: notifications(),
                })
            },
        )
        .expect("window");
        cx.activate(true);
    });
}
