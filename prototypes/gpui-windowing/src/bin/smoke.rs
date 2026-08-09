//! Drives one real GPUI window through the Longhorn host adapter.
//!
//! Card 163's evidence stopped at "the seam compiles against `gpui`". This
//! closes the rest: it opens an actual window from a shared
//! `longhorn-windowing` plan, observes it, exercises the display-facts refusal
//! and its resolution, drives the maximize toggle, and closes it — then quits.
//!
//! It prints one JSON receipt and exits. Run it on an attended macOS session:
//!
//! ```sh
//! cd prototypes/gpui-windowing && cargo run --bin smoke
//! ```
//!
//! It is not in `effigy qa` and cannot be: it needs a window server.

use gpui::{App, Application};
use longhorn_core::{ScaleFactor, ScreenPoint, ScreenRect, ScreenSize, WindowId, WindowPlacement};
use longhorn_gpui_windowing::{
    GpuiApplyOutcome, GpuiDisplayFacts, GpuiDisplayFactsSource, GpuiDisplayObservation,
    GpuiWindowBackend, GpuiWindowRegistry, execute_gpui_window_apply, gpui_host_capabilities,
};
use longhorn_gpui_windowing_prototype::GpuiAppBackend;
use longhorn_windowing::{ApplyGeneration, DesiredWindow, WindowDiffInput, WindowOperationKind};

/// Supplies whatever the caller has learned from a live window, and nothing
/// it has not.
#[derive(Default)]
struct LearnedDisplayFacts {
    scale: Option<ScaleFactor>,
}

impl GpuiDisplayFactsSource for LearnedDisplayFacts {
    fn scale_factor(&mut self, _facts: &GpuiDisplayFacts) -> Option<ScaleFactor> {
        self.scale
    }

    fn work_area(&mut self, facts: &GpuiDisplayFacts) -> Option<ScreenRect> {
        // A real product insets for the menu bar and dock from platform APIs
        // gpui does not expose. Standing in for that here with full bounds is
        // a deliberate choice this binary states, not a default the adapter
        // takes on anyone's behalf.
        self.scale?;
        facts.bounds().to_screen_rect().ok()
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
    let window_id = WindowId::new("smoke").expect("literal window id is valid");
    let desired = vec![DesiredWindow::new(
        window_id.clone(),
        WindowPlacement::new(ScreenPoint::new(160, 120), ScreenSize::new(720, 480)),
        false,
        true,
    )];
    let input = WindowDiffInput::new(
        desired.clone(),
        Vec::new(),
        gpui_host_capabilities(true),
        ApplyGeneration::new(1),
    );

    let mut lines: Vec<String> = Vec::new();
    let mut backend = GpuiAppBackend::new(cx);
    let mut displays = LearnedDisplayFacts::default();

    // 1. Create from the shared plan, with no display facts to hand.
    let bundle = match execute_gpui_window_apply(
        input,
        GpuiWindowRegistry::default(),
        &mut backend,
        &mut displays,
    ) {
        Ok(bundle) => bundle,
        Err(error) => return format!("{{\"ok\":false,\"stage\":\"apply\",\"error\":\"{error}\"}}"),
    };

    let created = bundle
        .receipt()
        .attempts()
        .iter()
        .find(|attempt| attempt.operation() == WindowOperationKind::Create)
        .is_some_and(|attempt| matches!(attempt.outcome(), GpuiApplyOutcome::Succeeded { .. }));
    lines.push(format!("\"created\":{created}"));
    lines.push(format!(
        "\"desired_state_reached\":{}",
        bundle.desired_state_reached()
    ));
    lines.push(format!(
        "\"dispositions\":{}",
        bundle.dispositions().len()
    ));

    let registry = bundle.into_parts().0;
    let Some(managed) = registry.managed_windows().into_iter().next() else {
        return format!("{{\"ok\":false,\"stage\":\"registry\",{}}}", lines.join(","));
    };
    let key = managed.key();

    // 2. Observe the real window.
    let facts = match backend.observe(key) {
        Ok(facts) => facts,
        Err(error) => {
            return format!(
                "{{\"ok\":false,\"stage\":\"observe\",\"error\":\"{error}\",{}}}",
                lines.join(",")
            );
        }
    };
    lines.push(format!("\"observed_scale\":{}", facts.scale()));
    lines.push(format!(
        "\"observed_origin\":[{},{}]",
        facts.bounds().to_screen_origin().map(|p| p.x().get()).unwrap_or(i32::MIN),
        facts.bounds().to_screen_origin().map(|p| p.y().get()).unwrap_or(i32::MIN)
    ));

    // 3. Display facts: refused before the scale is known, resolved after.
    //    Counts are reported alongside the probe size, and a probe error is
    //    reported rather than folded into a zero — a silent zero here would
    //    read as "gpui reported no displays" when it might mean "the probe
    //    failed", and those are different findings.
    lines.push(format!(
        "\"gpui_display_count\":{}",
        backend.displays().map(|d| d.len() as i64).unwrap_or(-1)
    ));
    match longhorn_gpui_windowing::observe_gpui_displays(&mut backend, &mut displays) {
        Ok(observed) => lines.push(format!(
            "\"displays_refused_without_scale\":{}",
            observed
                .iter()
                .filter(|display| matches!(display, GpuiDisplayObservation::Unobtainable { .. }))
                .count()
        )),
        Err(error) => lines.push(format!("\"displays_probe_error\":\"{error}\"")),
    }

    displays.scale = facts.scale_factor().ok();
    match longhorn_gpui_windowing::observe_gpui_displays(&mut backend, &mut displays) {
        Ok(observed) => lines.push(format!(
            "\"displays_resolved_with_scale\":{}",
            observed
                .iter()
                .filter_map(GpuiDisplayObservation::resolved)
                .count()
        )),
        Err(error) => lines.push(format!("\"displays_resolved_error\":\"{error}\"")),
    }

    // 4. The maximize toggle, against real AppKit. Reported, not asserted:
    //    macOS animates the zoom, so `is_maximized` may lag the call.
    let maximize = backend.set_maximized(key, true).is_ok();
    let observed_maximized = backend
        .observe(key)
        .map(|facts| facts.bounds_state().is_maximized())
        .unwrap_or_default();
    lines.push(format!("\"maximize_call_ok\":{maximize}"));
    lines.push(format!("\"maximized_observed\":{observed_maximized}"));
    let _ = backend.set_maximized(key, false);

    // 5. Close.
    let closed = backend.close(key).is_ok();
    lines.push(format!("\"closed\":{closed}"));

    format!("{{\"ok\":true,{}}}", lines.join(","))
}
