//! The assembly from `docs/guides/gpui-composition.md`, as something that
//! compiles.
//!
//! Read this against the guide. The order below is the guide's composition
//! order, and if the two ever diverge one of them is wrong.
//!
//! What this is not: a demonstration of six domains. The guide is the surface;
//! this is the proof the guide's assembly holds. One domain — notifications —
//! goes end to end, because it needs a ledger and nothing else.
//!
//! ```sh
//! cd prototypes/gpui-composition && cargo run
//! ```

mod drag;
mod lifecycle;
mod store;

use std::sync::atomic::{AtomicU64, Ordering};

use gpui::{
    App, AppContext, Application, Bounds, Context, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Render, Styled, Window, WindowBounds, WindowOptions, div, point, px, size,
};
use longhorn_core::{
    HostServices, NotificationAuthorityId, NotificationId, NotificationSourceId, WindowId,
};
use longhorn_gpui_windowing::WITHHELD_CAPABILITIES;
use longhorn_notifications::{
    NotificationAdd, NotificationAuthorityEpoch, NotificationDraft, NotificationLedger,
    NotificationLedgerLimits, NotificationSeverity, NotificationSummary, NotificationTitle,
};
use longhorn_poodle::project_notification_stack;
use longhorn_transfer::TransferClientId;
use poodle_adapter::ThemeProvider;
use poodle_gpui::GpuiThemeProvider;
use poodle_render::toast_stack::{self, ToastStackHandlers};

// ---------------------------------------------------------------------------
// Step 3. Supply `HostServices`, before anything that formats or folds.
// ---------------------------------------------------------------------------

/// The three platform facilities a webview gives away and GPUI does not.
///
/// Deliberately not `PlainHostServices`. That exists for tests and is named to
/// discourage exactly this; an application that ships it is telling its users
/// that dates look like integers.
struct ExampleHost {
    requests: AtomicU64,
}

impl ExampleHost {
    const fn new() -> Self {
        Self {
            requests: AtomicU64::new(1),
        }
    }
}

impl HostServices for ExampleHost {
    fn new_request_id(&self) -> String {
        // A counter, because this example is single-process and short-lived.
        // A real application wants something unique across restarts — a UUID,
        // or a counter seeded from durable state. Longhorn does not choose;
        // it only requires that two calls never agree.
        format!("example:{}", self.requests.fetch_add(1, Ordering::Relaxed))
    }

    fn format_timestamp(&self, unix_seconds: i64) -> String {
        // Written out rather than pulled from a date crate, because *which*
        // date crate is the application's decision and the guide's point is
        // that Longhorn does not make it. A real product picks one and gets
        // time zones and locale with it; this gets UTC and English.
        let (year, month, day) = civil_from_unix_day(unix_seconds.div_euclid(86_400));
        const MONTHS: [&str; 12] = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        format!("{day} {} {year}", MONTHS[(month - 1) as usize])
    }

    fn fold_case(&self, value: &str) -> String {
        // Unicode default. A product shipping in a locale with its own casing
        // rules — Turkish dotless i is the usual example — overrides here, and
        // this is the only place it has to.
        value.to_lowercase()
    }
}

/// Days since the Unix epoch to a civil date.
///
/// Howard Hinnant's `civil_from_days`, which is exact for the whole range and
/// short enough to read. Present only so `format_timestamp` has something
/// honest to do.
fn civil_from_unix_day(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = u32::try_from(day_of_year - (153 * shifted_month + 2) / 5 + 1).unwrap_or(1);
    let month = u32::try_from(if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    })
    .unwrap_or(1);

    (year + i64::from(month <= 2), month, day)
}

// ---------------------------------------------------------------------------
// Steps 1 and 2. Domain authority. A ledger is the whole domain here.
// ---------------------------------------------------------------------------

/// Publishes one notification per severity through the domain's own path.
///
/// Through `NotificationLedger` rather than by constructing records, because a
/// projection is only interesting if what it projects came through admission.
fn ledger() -> NotificationLedger {
    let mut ledger = NotificationLedger::new(
        NotificationAuthorityId::new("notifications:example").expect("authority"),
        NotificationAuthorityEpoch::new(1).expect("epoch"),
        NotificationLedgerLimits::new(64, 4 * 1_024 * 1_024).expect("limits"),
    );

    for (index, (severity, title, summary)) in [
        (
            NotificationSeverity::Success,
            "Composition assembled",
            "Host services, window backend, projection, renderer.",
        ),
        (
            NotificationSeverity::Warning,
            "One display observed",
            "Scale and origin came from the platform reader, not from gpui.",
        ),
        (
            NotificationSeverity::Critical,
            "Severity says itself",
            "Critical shares the danger tone with Error, so the title carries it.",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let draft = NotificationDraft::new(
            NotificationSourceId::new("example").expect("source"),
            severity,
            NotificationTitle::new(title).expect("title"),
            NotificationSummary::new(summary).expect("summary"),
        );
        let add = NotificationAdd::new(
            ledger.authority().clone(),
            ledger.revision(),
            NotificationId::new(format!("example:{index}")).expect("id"),
            draft,
        );
        ledger.add(add).expect("publish");
    }

    ledger
}

// ---------------------------------------------------------------------------
// Step 6. Project, render, draw.
// ---------------------------------------------------------------------------

struct CompositionRoot {
    theme: GpuiThemeProvider,
    ledger: NotificationLedger,
    services: ExampleHost,
    /// Which window this root is, so a press knows where it came from.
    window_id: WindowId,
}

impl Render for CompositionRoot {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        poodle_gpui_node_backend::reset_element_ids();
        let theme = &self.theme;

        let records: Vec<_> = self.ledger.records().cloned().collect();
        // `project_notification_stack`, not `project_notifications`: a `Toast`
        // is a leaf and the stack is what renders.
        let stack = project_notification_stack(&records);

        let canvas =
            poodle_gpui_node_backend::color(theme.resolve_color("color.background.canvas"));
        let text = poodle_gpui_node_backend::color(theme.resolve_color("color.text.primary"));

        let source = self.window_id.clone();
        let outcome = drag::TransferState::outcome(_cx);

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(20.0))
            .bg(canvas)
            .text_color(text)
            // Step 3's proof: this date is `ExampleHost::format_timestamp`,
            // not `Timestamp`'s own `Display`.
            .child(format!(
                "{} — assembled {}",
                self.window_id,
                self.services.format_timestamp(1_786_320_000)
            ))
            // Contract 020's last ceiling, live. Press here and release over
            // the other window; `on_mouse_up` fires on *this* window even when
            // the cursor is elsewhere, because macOS routes a drag's events to
            // the window that received the press.
            .child(
                div()
                    .id("transfer-source")
                    .p(px(12.0))
                    .border_1()
                    .border_color(text)
                    .child("drag me to the other window")
                    .on_mouse_down(MouseButton::Left, {
                        let source = source.clone();
                        move |_event, _window, cx| drag::TransferState::press(cx, &source)
                    })
                    .on_mouse_up(MouseButton::Left, move |event, window, cx| {
                        let released = drag::screen_point_of(window, event.position);
                        drag::TransferState::release(cx, released);
                    }),
            )
            .child(outcome)
            .child(drag::TransferState::persistence(_cx))
            .child(format!(
                "withheld by gpui: {}",
                WITHHELD_CAPABILITIES
                    .iter()
                    .map(|withheld| format!("{:?}", withheld.capability))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .child(poodle_gpui_node_backend::to_gpui(
                &toast_stack::toast_stack(&stack, theme, ToastStackHandlers::default()),
            ))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        // Two windows, side by side, because one window cannot prove a
        // cross-window drag. Step 4 and 5 — the window backend and lifecycle
        // host — are the neighbouring `gpui-windowing` prototype's subject;
        // this example opens through gpui directly and borrows that
        // prototype's backend only at release, to observe.
        let mut managed = Vec::new();

        for (index, origin) in [point(px(120.0), px(120.0)), point(px(760.0), px(120.0))]
            .into_iter()
            .enumerate()
        {
            let window_id = WindowId::new(format!("window:{index}")).expect("window id");
            let bounds = Bounds {
                origin,
                size: size(px(560.0), px(560.0)),
            };
            let root_id = window_id.clone();
            let handle = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        ..Default::default()
                    },
                    |_, cx| {
                        cx.new(|_| CompositionRoot {
                            theme: GpuiThemeProvider::new(),
                            ledger: ledger(),
                            services: ExampleHost::new(),
                            window_id: root_id,
                        })
                    },
                )
                .expect("window");

            managed.push(drag::ManagedWindow {
                window_id,
                client_id: TransferClientId::new(format!("client:{index}")).expect("client id"),
                handle: handle.into(),
            });
        }

        // Card 176: the lifecycle host over the same two windows and the same
        // real store, with every close answered through Longhorn.
        lifecycle::install(
            managed
                .iter()
                .map(|window| (window.window_id.clone(), window.handle))
                .collect(),
            cx,
        );

        drag::TransferState::install(managed, cx);
        cx.activate(true);
    });
}
