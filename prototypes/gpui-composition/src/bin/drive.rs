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
            eprintln!("[drive] drag {:?} -> {:?}", (from.x, from.y), (to.x, to.y));
            drag(from, to);
        }
        Some("click") => {
            let at = CGPoint::new(number(1), number(2));
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
