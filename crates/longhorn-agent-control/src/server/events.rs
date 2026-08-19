//! Observability resources that ride `subscriptions/listen`.
//!
//! rmcp 3.1.3's listen sink admits only the MCP change notifications
//! (list-changed and `resources/updated`). Custom notifications and
//! logging are rejected. Console, page-error, and navigation events
//! therefore live as three resources; a listen subscriber opts in by URI
//! and receives `notifications/resources/updated` on that stream. The
//! payload (events + drop counter) is the resource body.

use rmcp::model::Resource;

/// Console messages captured by the in-page shim.
pub const CONSOLE_URI: &str = "longhorn://agent-control/console";
/// Uncaught page errors and unhandled rejections.
pub const ERROR_URI: &str = "longhorn://agent-control/error";
/// Same-document navigations (pushState, replaceState, hashchange, popstate).
pub const NAVIGATION_URI: &str = "longhorn://agent-control/navigation";

pub fn all_resources() -> Vec<Resource> {
    vec![
        Resource::new(CONSOLE_URI, "console")
            .with_description("Console output captured in the page")
            .with_mime_type("application/json"),
        Resource::new(ERROR_URI, "page-error")
            .with_description("Uncaught page errors and unhandled rejections")
            .with_mime_type("application/json"),
        Resource::new(NAVIGATION_URI, "navigation")
            .with_description("In-page navigation events")
            .with_mime_type("application/json"),
    ]
}

pub fn known_uri(uri: &str) -> bool {
    matches!(uri, CONSOLE_URI | ERROR_URI | NAVIGATION_URI)
}

pub fn kind_for_uri(uri: &str) -> Option<&'static str> {
    match uri {
        CONSOLE_URI => Some("console"),
        ERROR_URI => Some("error"),
        NAVIGATION_URI => Some("navigation"),
        _ => None,
    }
}

pub fn uri_for_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "console" => Some(CONSOLE_URI),
        "error" => Some(ERROR_URI),
        "navigation" => Some(NAVIGATION_URI),
        _ => None,
    }
}
