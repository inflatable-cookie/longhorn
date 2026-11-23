//! Wire argument adapters for MCP tools whose wire shape differs from the
//! core vocabulary.
//!
//! Almost every tool deserializes directly into its vocabulary `*Request`
//! type in [`crate::tools`], which derives `JsonSchema`. Only `resize_window`
//! needs a separate wire shape (flat `width`/`height` rather than nested
//! [`ClientSize`]).

use longhorn_core::{ClientSize, WindowId};
use rmcp::{ErrorData, schemars::JsonSchema};
use serde::Deserialize;

use crate::ResizeWindowRequest;

/// Invalid wire input; surfaced as a JSON-RPC invalid-params error.
pub(super) fn invalid_params(message: impl Into<String>) -> ErrorData {
    ErrorData::invalid_params(message.into(), None)
}

/// `resize_window` arguments: flat `width`/`height` on the wire, validated
/// into [`ClientSize`] for [`ResizeWindowRequest`].
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ResizeWindowArgs {
    /// Window id to resize.
    pub window: String,
    /// New content width in logical pixels.
    pub width: f64,
    /// New content height in logical pixels.
    pub height: f64,
}

impl ResizeWindowArgs {
    /// Validates into the vocabulary request.
    pub fn into_request(self) -> Result<ResizeWindowRequest, ErrorData> {
        let window = WindowId::new(self.window.clone())
            .map_err(|_| invalid_params(format!("invalid window id {:?}", self.window)))?;
        let size = ClientSize::new(self.width, self.height)
            .map_err(|error| invalid_params(format!("invalid window size: {error}")))?;
        Ok(ResizeWindowRequest { window, size })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ClickRequest, MAX_WAIT_TIMEOUT_MS, SetFileInputRequest, SnapshotRequest, WaitForRequest,
        WebviewLabel,
    };

    #[test]
    fn omitted_webview_deserializes_to_todays_meaning() {
        let from_empty: SnapshotRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(from_empty, SnapshotRequest::default());
        let click: ClickRequest = serde_json::from_str(r#"{"element":"e1"}"#).unwrap();
        assert_eq!(click.window, None);
        assert_eq!(click.webview, None);
        assert_eq!(click.element.as_str(), "e1");
    }

    #[test]
    fn webview_field_is_accepted_on_semantic_requests() {
        let snapshot: SnapshotRequest = serde_json::from_str(r#"{"webview":"preview"}"#).unwrap();
        assert_eq!(
            snapshot.webview.as_ref().map(WebviewLabel::as_str),
            Some("preview")
        );
        let click: ClickRequest =
            serde_json::from_str(r#"{"element":"e1","webview":"preview"}"#).unwrap();
        assert_eq!(click.webview.unwrap().as_str(), "preview");
    }

    #[test]
    fn set_file_input_request_validates_and_rejects_a_path_field() {
        let request: SetFileInputRequest = serde_json::from_str(
            r#"{"element":"e1","files":[{"name":"note.txt","contentBase64":"aGk="}]}"#,
        )
        .unwrap();
        request.validate().unwrap();
        assert_eq!(request.element.as_str(), "e1");
        assert_eq!(request.files[0].name, "note.txt");

        assert!(
            serde_json::from_str::<SetFileInputRequest>(
                r#"{"element":"e1","files":[{"name":"note.txt","contentBase64":"aGk="}],"path":"/tmp/x"}"#
            )
            .is_err()
        );
        let empty: SetFileInputRequest =
            serde_json::from_str(r#"{"element":"e1","files":[]}"#).unwrap();
        assert!(empty.validate().is_err());
        let bad: SetFileInputRequest = serde_json::from_str(
            r#"{"element":"e1","files":[{"name":"note.txt","contentBase64":"@@@"}]}"#,
        )
        .unwrap();
        assert!(bad.validate().is_err());
    }

    #[test]
    fn wait_for_timeout_ms_enforces_upper_bound() {
        let ok: WaitForRequest = serde_json::from_str(&format!(
            r#"{{"predicate":{{"predicate":"refResolve","element":"e1"}},"timeoutMs":{MAX_WAIT_TIMEOUT_MS}}}"#
        ))
        .unwrap();
        ok.validate().unwrap();
        let over: WaitForRequest = serde_json::from_str(&format!(
            r#"{{"predicate":{{"predicate":"refResolve","element":"e1"}},"timeoutMs":{}}}"#,
            MAX_WAIT_TIMEOUT_MS + 1
        ))
        .unwrap();
        assert!(over.validate().is_err());
    }

    #[test]
    fn resize_window_args_validates_window_and_size() {
        let args: ResizeWindowArgs =
            serde_json::from_str(r#"{"window":"main","width":800,"height":600}"#).unwrap();
        let request = args.into_request().unwrap();
        assert_eq!(request.window.as_str(), "main");
        assert_eq!(request.size.width().get(), 800.0);
        assert_eq!(request.size.height().get(), 600.0);

        let bad_size: ResizeWindowArgs =
            serde_json::from_str(r#"{"window":"main","width":-1,"height":600}"#).unwrap();
        assert!(bad_size.into_request().is_err());
    }
}
