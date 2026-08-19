//! Main-thread bridge to the window's `WKWebView`.
//!
//! The MCP tool handlers run on a tokio worker thread; every `WKWebView`
//! call must run on the app main thread. Each entry point here hands a
//! closure to `WebviewWindow::with_webview` (which tauri dispatches onto the
//! main thread), the closure issues the `WKWebView` call with a
//! `block2::RcBlock` completion handler, and the completion handler sends
//! the outcome through a `tokio::sync::oneshot` channel that the async
//! caller awaits.

use std::sync::{Arc, Mutex};

use block2::RcBlock;
use objc2::{msg_send, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage};
use objc2_foundation::{NSError, NSNumber, NSString};
use objc2_web_kit::WKWebView;
use tauri::WebviewWindow;
use tokio::sync::oneshot;

/// Runs `js` in the webview and returns the result as plain text.
pub async fn evaluate(window: &WebviewWindow, js: String) -> Result<String, String> {
    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));
    window
        .with_webview(move |webview| {
            let webview = retain_webview(&webview);
            let source = NSString::from_str(&js);
            let tx = Arc::clone(&tx);
            let completion = RcBlock::new(move |result: *mut AnyObject, error: *mut NSError| {
                let outcome = match error_result(error) {
                    Some(error) => Err(error),
                    None => Ok(stringify_result(result)),
                };
                send(&tx, outcome);
            });
            // SAFETY: runs on the main thread; `webview` is a live WKWebView
            // and `completion` matches the selector's block signature.
            unsafe {
                webview.evaluateJavaScript_completionHandler(&source, Some(&completion));
            }
        })
        .map_err(|error| format!("with_webview dispatch failed: {error}"))?;
    rx.await
        .map_err(|_| "main thread dropped the evaluate result".to_string())?
}

/// Takes a fresh `WKWebView` snapshot and returns it as PNG bytes.
pub async fn screenshot(window: &WebviewWindow) -> Result<Vec<u8>, String> {
    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));
    window
        .with_webview(move |webview| {
            let webview = retain_webview(&webview);
            let completion = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
                let outcome = match error_result(error) {
                    Some(error) => Err(error),
                    None if image.is_null() => Err("takeSnapshot returned a nil image".to_string()),
                    // SAFETY: `image` is non-null and owned by the block
                    // call frame, which outlives this body.
                    None => png_bytes(unsafe { &*image }),
                };
                send(&tx, outcome);
            });
            // SAFETY: runs on the main thread; nil configuration asks for a
            // snapshot of the current viewport, and `completion` matches the
            // selector's block signature.
            unsafe {
                webview.takeSnapshotWithConfiguration_completionHandler(None, &completion);
            }
        })
        .map_err(|error| format!("with_webview dispatch failed: {error}"))?;
    rx.await
        .map_err(|_| "main thread dropped the screenshot result".to_string())?
}

/// Retains the raw `WKWebView` pointer handed over by tauri.
fn retain_webview(webview: &tauri::webview::PlatformWebview) -> Retained<WKWebView> {
    // SAFETY: tauri guarantees the pointer is a live WKWebView for the
    // duration of the `with_webview` closure.
    unsafe { Retained::retain(webview.inner().cast::<WKWebView>()) }
        .expect("with_webview must hand over a non-null WKWebView")
}

/// Converts an `NSError` pointer into a message, if present.
fn error_result(error: *mut NSError) -> Option<String> {
    // SAFETY: non-null pointers from a completion handler are live NSErrors.
    (!error.is_null()).then(|| unsafe { &*error }.localizedDescription().to_string())
}

/// Sends the outcome once, ignoring a dropped receiver.
fn send<T>(tx: &Mutex<Option<oneshot::Sender<T>>>, outcome: T) {
    if let Some(tx) = tx.lock().expect("result sender poisoned").take() {
        let _ = tx.send(outcome);
    }
}

/// Renders a JavaScript result value as text: strings and numbers as plain
/// text, anything else via its ObjC `description`, nil as `"undefined"`.
fn stringify_result(result: *mut AnyObject) -> String {
    if result.is_null() {
        // WKWebView reports both `undefined` and `null` results as nil.
        return "undefined".to_string();
    }
    // SAFETY: non-null result pointers are live objects for the duration of
    // the completion handler.
    let object = unsafe { &*result };
    if let Some(string) = object.downcast_ref::<NSString>() {
        return string.to_string();
    }
    if let Some(number) = object.downcast_ref::<NSNumber>() {
        return number.to_string();
    }
    // SAFETY: every ObjC object responds to `description`.
    let description: Retained<NSString> = unsafe { msg_send![object, description] };
    description.to_string()
}

/// Converts an `NSImage` to PNG bytes via TIFF → bitmap rep → PNG.
fn png_bytes(image: &NSImage) -> Result<Vec<u8>, String> {
    let tiff = image
        .TIFFRepresentation()
        .ok_or_else(|| "TIFFRepresentation returned nil".to_string())?;
    let rep = NSBitmapImageRep::imageRepWithData(&tiff)
        .ok_or_else(|| "NSBitmapImageRep could not decode the TIFF data".to_string())?;
    let properties = objc2_foundation::NSDictionary::new();
    // SAFETY: an empty properties dictionary is valid for PNG output.
    let png =
        unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties) }
            .ok_or_else(|| "PNG encoding returned nil".to_string())?;
    Ok(png.to_vec())
}
