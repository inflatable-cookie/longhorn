//! Main-thread bridge to a window's `WKWebView`: `screenshot` capture and
//! the `evaluate` escape hatch (Card 231).
//!
//! Mechanics follow the Card 227 spike's proven shape — the tool handlers
//! run on the control server's tokio threads, every `WKWebView` call must
//! run on the app main thread, so each entry point hands a closure to
//! `Webview::with_webview` (tauri dispatches it onto the main
//! thread), the closure issues the `WKWebView` call with a `block2`
//! completion handler, and the completion sends the outcome through a
//! `tokio::sync::oneshot` the async caller awaits. What changed from the
//! donor: every failure is a typed core-vocabulary error — no `expect` on
//! the main thread, ever.
//!
//! Capture is public API only: `takeSnapshotWithConfiguration:` with a nil
//! configuration (current viewport), PNG-encoded through
//! `NSBitmapImageRep`. No entitlement, no screen-recording permission, no
//! private API — the packaged freshness matrix records that claim.

// The workspace posture is `unsafe_code = deny` for this crate; this module
// is the single scoped exception because the objc2 snapshot/evaluate/
// retain calls are `unsafe fn` by signature. Every call site carries its
// SAFETY argument.
#![allow(unsafe_code)]

use std::sync::{Arc, Mutex};

use block2::RcBlock;
use objc2::{msg_send, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage};
use objc2_foundation::{NSError, NSNumber, NSString};
use objc2_web_kit::WKWebView;
use serde_json::Value;
use tauri::{Runtime, Webview};
use tokio::sync::oneshot;

use longhorn_agent_control::ToolError;

/// Runs `js` in the window's webview and returns the JSON-shaped result:
/// strings as strings, numbers as numbers, `undefined`/`null` (both reported
/// as nil by `WKWebView`) as null, anything else via its ObjC `description`.
pub(crate) async fn evaluate_webview<R: Runtime>(
    webview_handle: &Webview<R>,
    js: String,
) -> Result<Value, ToolError> {
    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));
    webview_handle
        .with_webview(move |webview| {
            let tx = Arc::clone(&tx);
            let Some(webview) = retain_webview(&webview, &tx, |message| {
                ToolError::EvaluationFailed { message }
            }) else {
                return;
            };
            let source = NSString::from_str(&js);
            let completion = RcBlock::new(move |result: *mut AnyObject, error: *mut NSError| {
                let outcome = match error_message(error) {
                    Some(message) => Err(ToolError::EvaluationFailed { message }),
                    None => Ok(json_result(result)),
                };
                send(&tx, outcome);
            });
            // SAFETY: runs on the main thread inside `with_webview`;
            // `webview` is a live WKWebView and `completion` matches the
            // selector's block signature.
            unsafe {
                webview.evaluateJavaScript_completionHandler(&source, Some(&completion));
            }
        })
        .map_err(|error| ToolError::EvaluationFailed {
            message: format!("main-thread dispatch failed: {error}"),
        })?;
    rx.await.map_err(|_| ToolError::EvaluationFailed {
        message: "the main thread dropped the evaluate result".to_owned(),
    })?
}

/// Takes a fresh `WKWebView` snapshot of the window's current viewport and
/// returns it as PNG bytes.
pub(crate) async fn screenshot_webview<R: Runtime>(
    webview_handle: &Webview<R>,
) -> Result<Vec<u8>, ToolError> {
    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));
    webview_handle
        .with_webview(move |webview| {
            let tx = Arc::clone(&tx);
            let Some(webview) = retain_webview(&webview, &tx, capture_failed) else {
                return;
            };
            let completion = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
                let outcome = match error_message(error) {
                    Some(message) => Err(capture_failed(message)),
                    None if image.is_null() => {
                        Err(capture_failed("takeSnapshot returned a nil image"))
                    }
                    // SAFETY: `image` is non-null and owned by the block
                    // call frame, which outlives this body.
                    None => png_bytes(unsafe { &*image }),
                };
                send(&tx, outcome);
            });
            // SAFETY: runs on the main thread inside `with_webview`; a nil
            // configuration asks for a snapshot of the current viewport,
            // and `completion` matches the selector's block signature.
            unsafe {
                webview.takeSnapshotWithConfiguration_completionHandler(None, &completion);
            }
        })
        .map_err(|error| capture_failed(format!("main-thread dispatch failed: {error}")))?;
    rx.await
        .map_err(|_| capture_failed("the main thread dropped the screenshot result"))?
}

/// Retains the raw `WKWebView` pointer tauri hands over; on failure, reports
/// through the channel and returns `None` so the closure can simply return.
fn retain_webview<T>(
    webview: &tauri::webview::PlatformWebview,
    tx: &Mutex<Option<oneshot::Sender<Result<T, ToolError>>>>,
    failure: fn(String) -> ToolError,
) -> Option<Retained<WKWebView>> {
    // SAFETY: tauri guarantees the pointer is a live WKWebView for the
    // duration of the `with_webview` closure.
    let retained = unsafe { Retained::retain(webview.inner().cast::<WKWebView>()) };
    if retained.is_none() {
        send(
            tx,
            Err(failure(
                "with_webview handed over a nil WKWebView".to_owned(),
            )),
        );
    }
    retained
}

/// Converts an `NSError` pointer into a message, if present.
fn error_message(error: *mut NSError) -> Option<String> {
    // SAFETY: non-null pointers from a completion handler are live NSErrors.
    (!error.is_null()).then(|| unsafe { &*error }.localizedDescription().to_string())
}

/// Sends the outcome once, ignoring a dropped receiver.
fn send<T>(tx: &Mutex<Option<oneshot::Sender<T>>>, outcome: T) {
    if let Some(tx) = tx.lock().expect("result sender poisoned").take() {
        let _ = tx.send(outcome);
    }
}

/// Renders a JavaScript result value as JSON.
fn json_result(result: *mut AnyObject) -> Value {
    if result.is_null() {
        // WKWebView reports both `undefined` and `null` results as nil.
        return Value::Null;
    }
    // SAFETY: non-null result pointers are live objects for the duration of
    // the completion handler.
    let object = unsafe { &*result };
    if let Some(string) = object.downcast_ref::<NSString>() {
        return Value::String(string.to_string());
    }
    if let Some(number) = object.downcast_ref::<NSNumber>() {
        return serde_json::Number::from_f64(number.as_f64())
            .map_or_else(|| Value::String(number.to_string()), Value::Number);
    }
    // SAFETY: every ObjC object responds to `description`.
    let description: Retained<NSString> = unsafe { msg_send![object, description] };
    Value::String(description.to_string())
}

/// Converts an `NSImage` to PNG bytes via TIFF → bitmap rep → PNG.
fn png_bytes(image: &NSImage) -> Result<Vec<u8>, ToolError> {
    let tiff = image
        .TIFFRepresentation()
        .ok_or_else(|| capture_failed("TIFFRepresentation returned nil"))?;
    let rep = NSBitmapImageRep::imageRepWithData(&tiff)
        .ok_or_else(|| capture_failed("NSBitmapImageRep could not decode the TIFF data"))?;
    let properties = objc2_foundation::NSDictionary::new();
    // SAFETY: an empty properties dictionary is valid for PNG output.
    let png =
        unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties) }
            .ok_or_else(|| capture_failed("PNG encoding returned nil"))?;
    Ok(png.to_vec())
}

/// The vocabulary's closest fit for a capture-side failure: the surface
/// could not serve this request, with the reason in the message.
fn capture_failed(message: impl Into<String>) -> ToolError {
    ToolError::Unsupported {
        message: format!("screenshot capture failed: {}", message.into()),
    }
}
