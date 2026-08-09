//! Places two real GPUI windows from one shared plan, across two displays.
//!
//! Contract 020 records multi-window placement as unproven on **either**
//! backend, and Tier B of the g02 candidate runway wants a live multi-monitor
//! scale proof. Both need hardware the QA gate does not have, so this runs by
//! hand with a second display attached.
//!
//! ```sh
//! cd prototypes/gpui-windowing && cargo run --bin multiwindow
//! ```
//!
//! What it is after:
//!
//! - two windows planned in one `plan_window_diff` pass and applied together,
//!   which no test has done against a real host
//! - each window's live scale factor, because
//!   `GpuiDisplayFactsSource::scale_factor` is asked once per *display* while
//!   gpui only reports scale per *window*. With a 1x external screen and a 2x
//!   laptop panel, any single answer is wrong for one of them.
//! - `GpuiWindowCreateRequest::on_display`, which exists and has never run.

use gpui::{App, Application};
use longhorn_core::{ScreenPoint, ScreenSize, WindowId, WindowPlacement};
use longhorn_gpui_windowing::{
    GpuiApplyOutcome, GpuiWindowBackend, GpuiWindowCreateRequest, GpuiWindowKey,
    GpuiWindowRegistry, execute_gpui_window_apply, gpui_host_capabilities,
};
use longhorn_gpui_windowing_prototype::GpuiAppBackend;
use longhorn_windowing::{ApplyGeneration, DesiredWindow, WindowDiffInput, WindowOperationKind};

mod facts {
    use longhorn_core::{ScaleFactor, ScreenRect};
    use longhorn_gpui_windowing::{GpuiDisplayFacts, GpuiDisplayFactsSource};

    /// Supplies nothing. This run is about what gpui reports, not about what
    /// a product would inject on top of it.
    pub struct Bare;

    impl GpuiDisplayFactsSource for Bare {
        fn scale_factor(&mut self, _facts: &GpuiDisplayFacts) -> Option<ScaleFactor> {
            None
        }

        fn work_area(&mut self, _facts: &GpuiDisplayFacts) -> Option<ScreenRect> {
            None
        }
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.activate(true);
        let report = drive(cx);
        println!("{report}");
        cx.quit();
    });
}

fn drive(cx: &mut App) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut backend = GpuiAppBackend::new(cx);

    let displays = match backend.displays() {
        Ok(displays) => displays,
        Err(error) => {
            return format!("{{\"ok\":false,\"stage\":\"displays\",\"error\":\"{error}\"}}");
        }
    };
    lines.push(format!("\"displayCount\":{}", displays.len()));

    // Scale per display, read from CoreGraphics before any window exists.
    // gpui's `PlatformDisplay` reports no scale, which looks like it makes
    // this unknowable until something is placed there. It does not: gpui's
    // `DisplayId` is the `CGDirectDisplayID`, so the id it already hands over
    // is the key the platform wants.
    let windowless: Vec<String> = displays
        .iter()
        .map(|display| {
            format!(
                "{{\"displayId\":{},\"scaleWithoutAWindow\":{},\"gpuiOrigin\":[{},{}],\"realOrigin\":[{},{}]}}",
                display.display_id(),
                longhorn_gpui_windowing_prototype::display_scale_factor(display.display_id())
                    .map_or("null".to_owned(), |scale| scale.to_string()),
                display.bounds().to_screen_origin().map(|p| p.x().get()).unwrap_or(-1),
                display.bounds().to_screen_origin().map(|p| p.y().get()).unwrap_or(-1),
                longhorn_gpui_windowing_prototype::display_origin(display.display_id()).0,
                longhorn_gpui_windowing_prototype::display_origin(display.display_id()).1
            )
        })
        .collect();
    lines.push(format!("\"windowlessScales\":[{}]", windowless.join(",")));
    if displays.len() < 2 {
        lines.push(
            "\"note\":\"only one display attached; the interesting claims are skipped\"".to_owned(),
        );
    }

    // 1. Two windows, one plan. The planner has never produced a multi-window
    //    apply against a real host on either backend.
    let first = WindowId::new("left").expect("literal id");
    let second = WindowId::new("right").expect("literal id");
    let desired = vec![
        DesiredWindow::new(
            first.clone(),
            WindowPlacement::new(ScreenPoint::new(120, 140), ScreenSize::new(640, 420)),
            false,
            true,
        ),
        DesiredWindow::new(
            second.clone(),
            WindowPlacement::new(ScreenPoint::new(900, 200), ScreenSize::new(560, 380)),
            false,
            true,
        ),
    ];
    let input = WindowDiffInput::new(
        desired,
        Vec::new(),
        gpui_host_capabilities(true),
        ApplyGeneration::new(1),
    );

    let bundle = match execute_gpui_window_apply(
        input,
        GpuiWindowRegistry::default(),
        &mut backend,
        &mut facts::Bare,
    ) {
        Ok(bundle) => bundle,
        Err(error) => return format!("{{\"ok\":false,\"stage\":\"apply\",\"error\":\"{error}\"}}"),
    };

    let created = bundle
        .receipt()
        .attempts()
        .iter()
        .filter(|attempt| {
            attempt.operation() == WindowOperationKind::Create
                && matches!(attempt.outcome(), GpuiApplyOutcome::Succeeded { .. })
        })
        .count();
    lines.push(format!("\"windowsCreated\":{created}"));
    lines.push(format!(
        "\"multiWindowDesiredStateReached\":{}",
        bundle.desired_state_reached()
    ));

    let registry = bundle.into_parts().0;
    let managed: Vec<_> = registry.managed_windows();
    let observed: Vec<String> = managed
        .iter()
        .filter_map(|window| {
            let facts = backend.observe(window.key()).ok()?;
            let origin = facts.bounds().to_screen_origin().ok()?;
            Some(format!(
                "{{\"id\":\"{}\",\"origin\":[{},{}],\"scale\":{}}}",
                window.window_id().map_or("?", |id| id.as_str()),
                origin.x().get(),
                origin.y().get(),
                facts.scale()
            ))
        })
        .collect();
    lines.push(format!("\"placed\":[{}]", observed.join(",")));

    // 2. One window per display, targeted explicitly. `on_display` has never
    //    executed; the apply engine never sets it, because a pure placement
    //    plan has no display concept to set it from.
    let mut per_display: Vec<String> = Vec::new();
    let mut opened: Vec<GpuiWindowKey> = Vec::new();
    for (ordinal, display) in displays.iter().enumerate() {
        let bounds = display.bounds();
        let Ok(size) = bounds.to_screen_size() else {
            continue;
        };
        // Inset from the display's own extent. Origins are useless here —
        // gpui reports every display at (0, 0) — so this lands wherever the
        // window server decides, which is itself the finding.
        let request = GpuiWindowCreateRequest::new(longhorn_gpui_windowing::GpuiLogicalRect::new(
            60.0,
            60.0,
            (size.width() / 3).max(320) as f32,
            (size.height() / 3).max(240) as f32,
        ))
        .on_display(display.display_id());

        let id = WindowId::new(format!("probe-{ordinal}")).expect("generated id is valid");
        match backend.create(&id, &request) {
            Ok(key) => {
                opened.push(key);
                let scale = backend.observe(key).map_or(f32::NAN, |facts| facts.scale());
                per_display.push(format!(
                    "{{\"displayId\":{},\"primary\":{},\"requestedOnDisplay\":true,\"windowScale\":{}}}",
                    display.display_id(),
                    display.is_primary(),
                    scale
                ));
            }
            Err(error) => per_display.push(format!(
                "{{\"displayId\":{},\"error\":\"{error}\"}}",
                display.display_id()
            )),
        }
    }
    lines.push(format!("\"perDisplay\":[{}]", per_display.join(",")));

    // The claim this run exists to settle: does a single per-display scale
    // answer actually hold on a mixed-DPI desk?
    let scales: Vec<String> = per_display
        .iter()
        .filter_map(|entry| entry.split("\"windowScale\":").nth(1))
        .map(|tail| tail.trim_end_matches('}').to_owned())
        .collect();
    let distinct = {
        let mut sorted = scales.clone();
        sorted.sort();
        sorted.dedup();
        sorted.len()
    };
    lines.push(format!("\"distinctWindowScales\":{distinct}"));
    lines.push(format!("\"oneScalePerDisplayHolds\":{}", distinct <= 1));

    for key in opened {
        let _ = backend.close(key);
    }
    for window in managed {
        let _ = backend.close(window.key());
    }

    format!("{{\"ok\":true,{}}}", lines.join(","))
}
