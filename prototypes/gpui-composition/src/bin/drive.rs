//! Posts real mouse events through the macOS window server.
//!
//! Card 175's `Do Not` forbids *simulating* the events, and this is not that.
//! The concern there is calling handlers directly and skipping the path a real
//! gesture takes. `CGEventPost` goes to the window server, which routes to the
//! application exactly as it routes a human's gesture — including the mouse
//! capture that sends a drag's release to the window that received the press,
//! which is the whole mechanism under test. The application cannot tell the
//! difference, and there is no in-process shortcut anywhere in the path.
//!
//! It does need Accessibility permission for whatever runs it. Without that
//! macOS drops the events silently, which looks identical to an application
//! that ignored them — so a run that reports nothing means check permissions
//! before concluding anything about Longhorn.
//!
//! # It refuses to fire blind
//!
//! macOS routes a posted event by screen position, not by which application is
//! frontmost. Firing at coordinates without checking what is actually there
//! sends the gesture into whatever window happens to occupy them — which
//! happened twice while proving Card 175, both times into the operator's own
//! session.
//!
//! So every gesture checks two things first: the target application is
//! frontmost, and the point lies inside one of its windows. Both come from
//! System Events, which reads the accessibility tree rather than guessing.
//! A refusal exits non-zero and says which check failed.
//!
//! ```sh
//! cargo run --bin drive -- drag 200 215 1040 420
//! cargo run --bin drive -- drag 200 215 720 400     # bare desktop
//! cargo run --bin drive -- click 1040 420
//! ```

use core_graphics::{
    event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton},
    event_source::{CGEventSource, CGEventSourceStateID},
    geometry::CGPoint,
};

/// The application these gestures are meant for.
const TARGET: &str = "composition";

fn osascript(script: &str) -> Option<String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn frontmost() -> Option<String> {
    osascript(
        "tell application \"System Events\" to get name of first process whose frontmost is true",
    )
}

/// Every window of the target application, as `(x, y, width, height)`.
///
/// System Events reports position and size per window, flattened into one
/// comma-separated list. Four numbers per window, in order.
fn target_windows() -> Vec<(f64, f64, f64, f64)> {
    let script = format!(
        "tell application \"System Events\" to tell process \"{TARGET}\" to get {{position, size}} of every window"
    );
    let Some(raw) = osascript(&script) else {
        return Vec::new();
    };

    let numbers: Vec<f64> = raw
        .split(',')
        .filter_map(|part| part.trim().parse().ok())
        .collect();

    // `{position, size}` of N windows arrives as N positions then N sizes, so
    // the pairs are half a list apart rather than adjacent.
    let count = numbers.len() / 4;
    (0..count)
        .map(|index| {
            (
                numbers[index * 2],
                numbers[index * 2 + 1],
                numbers[count * 2 + index * 2],
                numbers[count * 2 + index * 2 + 1],
            )
        })
        .collect()
}

/// Refuses unless the point is somewhere this driver is allowed to click.
///
/// Two checks, and both matter. Frontmost alone is not enough — another
/// application's window can still sit over the point. Inside-a-window alone is
/// not enough either, because a window can be behind something else.
fn refuse_unless_safe(points: &[CGPoint]) {
    let front = frontmost().unwrap_or_default();
    if !front.contains(TARGET) {
        eprintln!("[drive] refusing: frontmost is {front:?}, not {TARGET:?}");
        std::process::exit(2);
    }

    let windows = target_windows();
    if windows.is_empty() {
        eprintln!("[drive] refusing: {TARGET} reports no windows");
        std::process::exit(2);
    }

    for point in points {
        let inside = windows.iter().any(|(x, y, width, height)| {
            point.x >= *x && point.x <= x + width && point.y >= *y && point.y <= y + height
        });
        if !inside {
            eprintln!(
                "[drive] refusing: ({}, {}) is outside every {TARGET} window: {windows:?}",
                point.x, point.y
            );
            std::process::exit(2);
        }
    }
}

fn source() -> CGEventSource {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .expect("an event source; check Accessibility permission")
}

fn post(event_type: CGEventType, at: CGPoint) {
    let event = CGEvent::new_mouse_event(source(), event_type, at, CGMouseButton::Left)
        .expect("a mouse event");
    event.post(CGEventTapLocation::HID);
}

fn settle() {
    // Long enough for gpui to process a frame between steps. A drag posted in
    // one instant is a drag the application may coalesce into nothing.
    std::thread::sleep(std::time::Duration::from_millis(60));
}

fn drag(from: CGPoint, to: CGPoint) {
    post(CGEventType::MouseMoved, from);
    settle();
    post(CGEventType::LeftMouseDown, from);
    settle();

    // Intermediate moves, because a press and a release at two points with
    // nothing between is not what a drag looks like to a window server.
    for step in 1..=6 {
        let fraction = f64::from(step) / 6.0;
        post(
            CGEventType::LeftMouseDragged,
            CGPoint::new(
                from.x + (to.x - from.x) * fraction,
                from.y + (to.y - from.y) * fraction,
            ),
        );
        settle();
    }

    post(CGEventType::LeftMouseUp, to);
    settle();
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let number = |index: usize| -> f64 {
        arguments
            .get(index)
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| panic!("argument {index} must be a number"))
    };

    match arguments.first().map(String::as_str) {
        Some("drag") => {
            let from = CGPoint::new(number(1), number(2));
            let to = CGPoint::new(number(3), number(4));
            // The release may legitimately land on bare desktop, which is one
            // of the cases under test, so only the press is required to be
            // inside a window. That is also the only point that decides where
            // the gesture is captured.
            refuse_unless_safe(&[from]);
            eprintln!("[drive] drag {:?} -> {:?}", (from.x, from.y), (to.x, to.y));
            drag(from, to);
        }
        Some("click") => {
            let at = CGPoint::new(number(1), number(2));
            refuse_unless_safe(&[at]);
            eprintln!("[drive] click {:?}", (at.x, at.y));
            post(CGEventType::MouseMoved, at);
            settle();
            post(CGEventType::LeftMouseDown, at);
            settle();
            post(CGEventType::LeftMouseUp, at);
        }
        _ => eprintln!("usage: drive drag <x1> <y1> <x2> <y2> | drive click <x> <y>"),
    }
}
