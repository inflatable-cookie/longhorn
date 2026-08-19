//! Card 232 injectable shim and the host-side marshalling onto it.
//!
//! The JS asset is the bundled `packages/longhorn/src/agent-control` IIFE.
//! Injection is gated with this module behind `dev`; the release-absence
//! scan forbids the asset markers in a featureless build.

use std::time::{Duration, Instant};

use longhorn_agent_control::{
    ActionReceipt, PageState, SemanticNode, ToolError, WaitForResult, WaitPredicate,
};
use serde_json::{Value, json};

/// Bundled in-page shim. Idempotent: a reloaded page re-runs it and the
/// install no-ops if the global is already present.
pub const SHIM_SOURCE: &str = include_str!("agent_control_shim.js");

const POLL: Duration = Duration::from_millis(50);

/// Wraps a shim call so the page has the shim and returns JSON text.
pub fn shim_call(expression: &str) -> String {
    format!("{SHIM_SOURCE}\nJSON.stringify(({expression}));")
}

pub fn snapshot_js() -> String {
    shim_call("globalThis.__longhornAgentControl.snapshot()")
}

pub fn click_js(element: &str) -> String {
    shim_call(&format!(
        "globalThis.__longhornAgentControl.click({})",
        Value::String(element.to_owned())
    ))
}

pub fn type_js(element: &str, text: &str) -> String {
    shim_call(&format!(
        "globalThis.__longhornAgentControl.type({}, {})",
        Value::String(element.to_owned()),
        Value::String(text.to_owned())
    ))
}

pub fn press_js(key: &str, modifiers: &[String], element: Option<&str>) -> String {
    let element = match element {
        Some(element) => Value::String(element.to_owned()),
        None => Value::Null,
    };
    shim_call(&format!(
        "globalThis.__longhornAgentControl.press({}, {}, {})",
        Value::String(key.to_owned()),
        json!(modifiers),
        element
    ))
}

pub fn scroll_js(delta_x: i32, delta_y: i32, element: Option<&str>) -> String {
    let element = match element {
        Some(element) => Value::String(element.to_owned()),
        None => Value::Null,
    };
    shim_call(&format!(
        "globalThis.__longhornAgentControl.scroll({delta_x}, {delta_y}, {element})"
    ))
}

pub fn drag_js(source: &str, target: &str) -> String {
    shim_call(&format!(
        "globalThis.__longhornAgentControl.drag({}, {})",
        Value::String(source.to_owned()),
        Value::String(target.to_owned())
    ))
}

pub fn wait_for_js(predicate: &WaitPredicate) -> String {
    let payload = serde_json::to_value(predicate).expect("wait predicate is JSON");
    shim_call(&format!(
        "globalThis.__longhornAgentControl.waitFor({payload})"
    ))
}

/// WKWebView returns a JSON string as an NSString, which the capture bridge
/// surfaces as `Value::String`. Objects would be unusable ObjC descriptions.
pub fn unwrap_eval_json(value: Value) -> Result<Value, ToolError> {
    match value {
        Value::String(text) => {
            serde_json::from_str(&text).map_err(|error| ToolError::EvaluationFailed {
                message: format!("shim JSON did not parse: {error}"),
            })
        }
        Value::Null => Err(ToolError::EvaluationFailed {
            message: "shim call returned undefined".to_owned(),
        }),
        other => Ok(other),
    }
}

fn decode_envelope(value: Value) -> Result<Value, ToolError> {
    let value = unwrap_eval_json(value)?;
    match value.get("ok") {
        Some(Value::Bool(false)) => {
            let error = value.get("error").cloned().unwrap_or(Value::Null);
            Err(
                serde_json::from_value(error).map_err(|error| ToolError::EvaluationFailed {
                    message: format!("shim error JSON did not parse: {error}"),
                })?,
            )
        }
        Some(Value::Bool(true)) => Ok(value),
        _ => Err(ToolError::EvaluationFailed {
            message: "shim result is missing the ok flag".to_owned(),
        }),
    }
}

pub fn decode_action(value: Value) -> Result<ActionReceipt, ToolError> {
    decode_envelope(value)?;
    Ok(ActionReceipt {})
}

pub fn decode_snapshot(value: Value) -> Result<(PageState, SemanticNode), ToolError> {
    let value = decode_envelope(value)?;
    let page = serde_json::from_value(value.get("page").cloned().unwrap_or(Value::Null)).map_err(
        |error| ToolError::EvaluationFailed {
            message: format!("snapshot page JSON did not parse: {error}"),
        },
    )?;
    let root = serde_json::from_value(value.get("root").cloned().unwrap_or(Value::Null)).map_err(
        |error| ToolError::EvaluationFailed {
            message: format!("snapshot root JSON did not parse: {error}"),
        },
    )?;
    Ok((page, root))
}

pub fn decode_wait(value: Value) -> Result<bool, ToolError> {
    let value = decode_envelope(value)?;
    value
        .get("holds")
        .and_then(Value::as_bool)
        .ok_or_else(|| ToolError::EvaluationFailed {
            message: "waitFor JSON is missing holds".to_owned(),
        })
}

/// Host-side `wait_for` pacing: the shim answers "holds now?"; timeout
/// lives here. No time-only predicate exists in the shim.
pub async fn poll_until<F, Fut>(timeout_ms: u32, mut check: F) -> Result<WaitForResult, ToolError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<bool, ToolError>>,
{
    let deadline = Instant::now() + Duration::from_millis(u64::from(timeout_ms));
    loop {
        if check().await? {
            return Ok(WaitForResult {});
        }
        let now = Instant::now();
        if now >= deadline {
            return Err(ToolError::WaitTimeout { timeout_ms });
        }
        let remaining = deadline.saturating_duration_since(now);
        tokio::time::sleep(remaining.min(POLL)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use longhorn_agent_control::ElementRef;
    use serde_json::json;

    #[test]
    fn decode_maps_unresolved_ref_onto_the_vocabulary() {
        let value = json!({
            "ok": false,
            "error": { "error": "unresolvedRef", "element": "e1" }
        });
        let error = decode_action(value).unwrap_err();
        assert_eq!(
            error,
            ToolError::UnresolvedRef {
                element: ElementRef::new("e1").unwrap(),
            }
        );
    }

    #[test]
    fn decode_snapshot_reads_tree_and_page() {
        let value = json!({
            "ok": true,
            "page": { "url": "https://app.example/", "title": "Proof" },
            "root": {
                "elementRef": "e1",
                "role": "document",
                "states": ["visible"],
                "children": []
            }
        });
        let (page, root) = decode_snapshot(value).unwrap();
        assert_eq!(page.title, "Proof");
        assert_eq!(root.role, "document");
        assert_eq!(root.element_ref.as_str(), "e1");
    }

    #[test]
    fn shim_source_carries_the_live_dom_ref_attr() {
        assert!(SHIM_SOURCE.contains("data-longhorn-agent-ref"));
        assert!(SHIM_SOURCE.contains("__longhornAgentControl"));
        assert!(!SHIM_SOURCE.contains("setTimeout("));
        assert!(!SHIM_SOURCE.contains("requestAnimationFrame("));
    }

    #[tokio::test]
    async fn poll_until_times_out_when_the_predicate_never_holds() {
        let error = poll_until(20, || async { Ok(false) }).await.unwrap_err();
        assert_eq!(error, ToolError::WaitTimeout { timeout_ms: 20 });
    }

    #[tokio::test]
    async fn poll_until_returns_when_the_predicate_holds() {
        let mut remaining = 2u8;
        let result = poll_until(200, || {
            remaining = remaining.saturating_sub(1);
            let holds = remaining == 0;
            async move { Ok(holds) }
        })
        .await
        .unwrap();
        assert_eq!(result, WaitForResult {});
    }
}
