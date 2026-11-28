//! Main-thread bridge to a window's `WKWebView`s: `screenshot` capture and
//! the `evaluate` escape hatch (Cards 231, 238).
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
//!
//! Card 238 composition: a window hosting child webviews (native-content
//! islands) is captured as one image of the whole logical window. Each
//! hosted webview snapshots its own viewport (`takeSnapshot` reaches only
//! the webview it is called on — sibling `WKWebView` pixels never appear,
//! the Figmatic adoption finding), and the snapshots are drawn into a
//! physical-pixel bitmap of the window's inner size: the UI webview at
//! (0, 0), each child at its tauri-reported physical position and size,
//! back to front in the z-order sampled from the view hierarchy at capture
//! time (label order breaks ties), clipped by the bitmap bounds. A hidden
//! webview contributes nothing, matching what the window shows. A snapshot
//! failure on any hosted visible webview fails the whole capture typed —
//! silently dropping a surface would repeat the black-island failure.
//! Orientation facts (bitmap row 0 is the PNG top; an unflipped bitmap
//! context draws y-up with content upright) were pinned by a live probe,
//! not assumed — see the Card 238 closeout.

// The workspace posture is `unsafe_code = deny` for this crate; this module
// is the single scoped exception because the objc2 snapshot/evaluate/
// retain calls are `unsafe fn` by signature. Every call site carries its
// SAFETY argument.
#![allow(unsafe_code)]

use std::cell::Cell;

use block2::RcBlock;
use objc2::{AllocAnyThread, msg_send, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSCalibratedRGBColorSpace, NSCompositingOperation,
    NSGraphicsContext, NSImage,
};
use objc2_core_foundation::{CFType, kCFBooleanFalse, kCFBooleanTrue};
use objc2_foundation::{
    NSArray, NSData, NSDictionary, NSError, NSNumber, NSPoint, NSRect, NSSize, NSString,
};
use objc2_web_kit::WKWebView;
use serde_json::Value;
use tauri::{Manager, PhysicalSize, Runtime, Webview, Window};
use tokio::sync::oneshot;

use longhorn_agent_control::ToolError;

use crate::handler::on_main_thread;

/// Runs `js` in the window's webview and returns the JSON-shaped result:
/// strings, numbers, and booleans keep their types. `undefined` and `null` (both reported
/// as nil by `WKWebView`) become null. Other values use their ObjC `description`.
pub(crate) async fn evaluate_webview<R: Runtime>(
    webview_handle: &Webview<R>,
    js: String,
) -> Result<Value, ToolError> {
    let (tx, rx) = oneshot::channel();
    dispatch_evaluation(webview_handle, js, tx)?;
    rx.await.map_err(|_| ToolError::EvaluationFailed {
        message: "the main thread dropped the evaluate result".to_owned(),
    })?
}

fn dispatch_evaluation<R: Runtime>(
    webview: &Webview<R>,
    js: String,
    tx: oneshot::Sender<Result<Value, ToolError>>,
) -> Result<(), ToolError> {
    #[cfg(test)]
    if let Some(dispatch) = webview.try_state::<tests::HeldWebviewDispatch>() {
        return dispatch.hold(tx);
    }

    webview
        .with_webview(move |webview| {
            if tx.is_closed() {
                return;
            }
            let Some(webview) = retain_webview(&webview) else {
                let _ = tx.send(Err(ToolError::EvaluationFailed {
                    message: "with_webview handed over a nil WKWebView".to_owned(),
                }));
                return;
            };
            let source = NSString::from_str(&js);
            let tx = Cell::new(Some(tx));
            let completion = RcBlock::new(move |result: *mut AnyObject, error: *mut NSError| {
                if let Some(tx) = tx.take() {
                    let outcome = match error_message(error) {
                        Some(message) => Err(ToolError::EvaluationFailed { message }),
                        None => Ok(json_result(result)),
                    };
                    let _ = tx.send(outcome);
                }
            });
            // SAFETY: this main-thread block matches the WKWebView selector.
            unsafe {
                webview.evaluateJavaScript_completionHandler(&source, Some(&completion));
            }
        })
        .map_err(|error| ToolError::EvaluationFailed {
            message: format!("main-thread dispatch failed: {error}"),
        })
}

/// One webview's contribution to the composed window image. Geometry is in
/// physical pixels relative to the window content area's top-left corner —
/// tauri reports child webview position and size in physical units against
/// exactly that origin, so no scale conversion enters the composition.
struct Surface {
    /// Webview label; breaks z-order ties deterministically.
    label: String,
    /// Back-to-front index within the superview's subview list, sampled at
    /// capture time; `usize::MAX` (drawn last) when the view is detached.
    order: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    /// PNG-encoded snapshot of this webview's current viewport.
    png: Vec<u8>,
}

/// Takes a fresh screenshot of the whole window: every hosted webview's
/// snapshot composed at its bounds into one image of the window's inner
/// size (Card 238). See the module docs for the composition rules.
pub(crate) async fn screenshot_window<R: Runtime>(
    window: &Window<R>,
) -> Result<Vec<u8>, ToolError> {
    let size_window = window.clone();
    let inner = on_main_thread(window.app_handle(), move || {
        size_window
            .inner_size()
            .map_err(|error| capture_failed(format!("window size read failed: {error}")))
    })
    .await?;
    let ui_label = window.label().to_owned();

    // Snapshot sequence is label-sorted for determinism; the draw order is
    // the sampled z-order, so snapshot sequencing cannot affect the image.
    let mut webviews = window.webviews();
    webviews.sort_by(|left, right| left.label().cmp(right.label()));
    let mut surfaces = Vec::new();
    for webview in webviews {
        let label = webview.label().to_owned();
        let (x, y, width, height) = if label == ui_label {
            // The UI webview fills the window; its `position()` reports the
            // window's screen coordinates, not a content-relative origin.
            (0.0, 0.0, f64::from(inner.width), f64::from(inner.height))
        } else {
            let child = webview.clone();
            on_main_thread(window.app_handle(), move || {
                let position = child.position().map_err(|error| {
                    capture_failed(format!(
                        "webview {:?} position read failed: {error}",
                        child.label()
                    ))
                })?;
                let size = child.size().map_err(|error| {
                    capture_failed(format!(
                        "webview {:?} size read failed: {error}",
                        child.label()
                    ))
                })?;
                Ok((
                    f64::from(position.x),
                    f64::from(position.y),
                    f64::from(size.width),
                    f64::from(size.height),
                ))
            })
            .await?
        };
        if let Some(surface) = snapshot_surface(&webview, label, x, y, width, height).await? {
            surfaces.push(surface);
        }
    }
    surfaces.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.label.cmp(&right.label))
    });
    on_main_thread(window.app_handle(), move || compose_png(inner, surfaces)).await
}

/// Snapshots one webview's viewport on the main thread. Returns `None` for
/// a hidden webview — it shows no pixels in the real window, so it shows
/// none in the image.
async fn snapshot_surface<R: Runtime>(
    webview_handle: &Webview<R>,
    label: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<Option<Surface>, ToolError> {
    let (tx, rx) = oneshot::channel();
    webview_handle
        .with_webview(move |webview| {
            if tx.is_closed() {
                return;
            }
            let Some(webview) = retain_webview(&webview) else {
                let _ = tx.send(Err(capture_failed(
                    "with_webview handed over a nil WKWebView",
                )));
                return;
            };
            if webview.isHiddenOrHasHiddenAncestor() {
                let _ = tx.send(Ok(None));
                return;
            }
            let order = z_order(&webview);
            let tx = Cell::new(Some(tx));
            let completion = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
                let Some(tx) = tx.take() else { return };
                let outcome = match error_message(error) {
                    Some(message) => Err(capture_failed(message)),
                    None if image.is_null() => {
                        Err(capture_failed("takeSnapshot returned a nil image"))
                    }
                    // SAFETY: `image` is non-null and owned by the block
                    // call frame, which outlives this body.
                    None => png_bytes(unsafe { &*image }).map(|png| {
                        Some(Surface {
                            label: label.clone(),
                            order,
                            x,
                            y,
                            width,
                            height,
                            png,
                        })
                    }),
                };
                let _ = tx.send(outcome);
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

/// The webview's back-to-front index among its superview's subviews;
/// `usize::MAX` when detached (sorts last, behind nothing real).
fn z_order(webview: &WKWebView) -> usize {
    // SAFETY: the webview is a live view on the main thread (inside
    // `with_webview`), so reading its superview is sound.
    let superview = unsafe { webview.superview() };
    let Some(superview) = superview else {
        return usize::MAX;
    };
    let subviews: Retained<NSArray<objc2_app_kit::NSView>> = superview.subviews();
    // NSNotFound is `NSUInteger::MAX` — the same "drawn last" sentinel.
    subviews.indexOfObjectIdenticalTo(webview)
}

/// The drawing itself: one bitmap at the window's physical size, each
/// surface's snapshot drawn at its bounds back to front, clipped by the
/// bitmap. The bitmap context is unflipped (y grows upward from the PNG's
/// bottom row), so the destination rect's y is mirrored; content draws
/// upright with `respectFlipped: false` — both facts pinned by live probe.
fn compose_png(inner: PhysicalSize<u32>, surfaces: Vec<Surface>) -> Result<Vec<u8>, ToolError> {
    // SAFETY: null planes ask AppKit to allocate the bitmap; the geometry
    // (8 bits per sample, 4 samples per pixel, non-planar calibrated RGB)
    // is a valid premultiplied RGBA layout, and the call runs on the main
    // thread.
    let canvas = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            inner.width.cast_signed() as isize,
            inner.height.cast_signed() as isize,
            8,
            4,
            true,
            false,
            NSCalibratedRGBColorSpace,
            0,
            32,
        )
    }
    .ok_or_else(|| capture_failed("the composition bitmap could not be allocated"))?;
    let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&canvas)
        .ok_or_else(|| capture_failed("no graphics context for the composition bitmap"))?;
    let previous = NSGraphicsContext::currentContext();
    NSGraphicsContext::setCurrentContext(Some(&context));
    let outcome = draw_surfaces(&canvas, surfaces);
    NSGraphicsContext::setCurrentContext(previous.as_deref());
    outcome?;
    encode_png(&canvas)
}

fn draw_surfaces(canvas: &NSBitmapImageRep, surfaces: Vec<Surface>) -> Result<(), ToolError> {
    let canvas_height = canvas.pixelsHigh() as f64;
    for surface in surfaces {
        let data = NSData::from_vec(surface.png);
        let rep = NSBitmapImageRep::imageRepWithData(&data).ok_or_else(|| {
            capture_failed(format!(
                "webview {:?} snapshot could not be decoded",
                surface.label
            ))
        })?;
        // The source rect must span the rep in its own coordinate space:
        // `size()` (points), not the pixel counts. A 2x snapshot decodes to
        // a rep whose point size is half its pixel count, and `drawInRect`
        // maps source points onto destination pixels linearly — using pixel
        // counts here draws the image into a corner quarter (the 1x/2x
        // compositing trap, caught by the packaged matrix).
        let source_size = rep.size();
        let source = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(source_size.width, source_size.height),
        );
        let destination = NSRect::new(
            NSPoint::new(surface.x, canvas_height - surface.y - surface.height),
            NSSize::new(surface.width, surface.height),
        );
        // SAFETY: main thread, both rects are well-formed, a nil hints
        // dictionary is valid, and `respectFlipped: false` matches the
        // unflipped bitmap context.
        let drawn = unsafe {
            rep.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                destination,
                source,
                NSCompositingOperation::SourceOver,
                1.0,
                false,
                None,
            )
        };
        if !drawn {
            return Err(capture_failed(format!(
                "webview {:?} snapshot could not be drawn",
                surface.label
            )));
        }
    }
    Ok(())
}

/// Retains the raw `WKWebView` pointer Tauri hands over, if present.
fn retain_webview(webview: &tauri::webview::PlatformWebview) -> Option<Retained<WKWebView>> {
    // SAFETY: tauri guarantees the pointer is a live WKWebView for the
    // duration of the `with_webview` closure.
    unsafe { Retained::retain(webview.inner().cast::<WKWebView>()) }
}

/// Converts an `NSError` pointer into a message, if present.
fn error_message(error: *mut NSError) -> Option<String> {
    // SAFETY: non-null pointers from a completion handler are live NSErrors.
    (!error.is_null()).then(|| unsafe { &*error }.localizedDescription().to_string())
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
        // SAFETY: the CFBoolean singletons are immutable framework constants.
        let booleans = unsafe { [kCFBooleanTrue, kCFBooleanFalse] };
        for boolean in booleans.into_iter().flatten() {
            let value: &CFType = boolean;
            if std::ptr::eq(object, value.as_ref()) {
                return Value::Bool(boolean.value());
            }
        }
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
    encode_png(&rep)
}

/// Encodes a bitmap rep as PNG.
fn encode_png(rep: &NSBitmapImageRep) -> Result<Vec<u8>, ToolError> {
    let properties = NSDictionary::new();
    // SAFETY: an empty properties dictionary is valid for PNG output.
    let png =
        unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties) }
            .ok_or_else(|| capture_failed("PNG encoding returned nil"))?;
    Ok(png.to_vec())
}

fn capture_failed(message: impl Into<String>) -> ToolError {
    ToolError::HostFailure {
        message: format!("screenshot capture failed: {}", message.into()),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write as _,
        net::TcpStream,
        sync::{Arc, mpsc},
        thread,
        time::{Duration, Instant},
    };

    use longhorn_agent_control::{ControlHandler as _, SnapshotRequest, enumerate_discovery};
    use tauri::{WebviewWindowBuilder, test::MockRuntime};

    use super::*;
    use crate::{
        AgentControlConfig, NoCommandBridge, handler::TauriControlHandler, mount_agent_control,
    };

    #[test]
    fn evaluate_keeps_booleans_distinct_from_numeric_zero_and_one() {
        for value in [false, true] {
            let boolean = NSNumber::new_bool(value);
            let number = NSNumber::new_i32(i32::from(value));
            assert_eq!(
                json_result(Retained::as_ptr(&boolean).cast_mut().cast()),
                Value::Bool(value)
            );
            assert_eq!(
                json_result(Retained::as_ptr(&number).cast_mut().cast()).as_f64(),
                Some(f64::from(i32::from(value)))
            );
        }
    }

    #[test]
    fn encoding_failure_is_a_host_failure() {
        assert!(matches!(
            capture_failed("PNG encoding returned nil"),
            ToolError::HostFailure { .. }
        ));
    }

    type EvaluationReply = oneshot::Sender<Result<Value, ToolError>>;

    /// The mock runtime drops `with_webview` callbacks. Hold the reply
    /// instead to reproduce a callback the main thread cannot complete.
    pub(super) struct HeldWebviewDispatch(mpsc::Sender<EvaluationReply>);

    impl HeldWebviewDispatch {
        pub(super) fn hold(&self, reply: EvaluationReply) -> Result<(), ToolError> {
            self.0.send(reply).map_err(|_| ToolError::EvaluationFailed {
                message: "the mock main thread dropped the dispatch queue".to_owned(),
            })
        }
    }

    fn app_with_held_dispatch() -> (tauri::App<MockRuntime>, mpsc::Receiver<EvaluationReply>) {
        let app = tauri::test::mock_app();
        WebviewWindowBuilder::new(app.handle(), "main", Default::default())
            .build()
            .unwrap();
        let (dispatch, callbacks) = mpsc::channel();
        assert!(app.manage(HeldWebviewDispatch(dispatch)));
        (app, callbacks)
    }

    #[test]
    fn shutdown_from_the_app_thread_does_not_wait_for_webkit() {
        let (webview_pending, pending) = mpsc::channel();
        let (shutdown_finished, finished) = mpsc::channel();
        let app_thread = thread::spawn(move || {
            let (app, callbacks) = app_with_held_dispatch();
            let state = tempfile::tempdir().unwrap();
            let discovery_dir = state.path().join("agent-control");
            let handle = mount_agent_control(
                app.handle(),
                AgentControlConfig::new("app").with_state_root(state.path().to_owned()),
                Arc::new(NoCommandBridge),
            )
            .unwrap();
            let ready_by = Instant::now() + Duration::from_secs(3);
            let record = loop {
                if let Some(record) = enumerate_discovery(&discovery_dir).unwrap().instances.pop() {
                    break record;
                }
                assert!(
                    Instant::now() < ready_by,
                    "server did not publish discovery"
                );
                thread::sleep(Duration::from_millis(10));
            };
            let file = record.file();
            let mut stream = TcpStream::connect(("127.0.0.1", file.port)).unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let body = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"snapshot","arguments":{"window":"main"}}}"#;
            write!(
                stream,
                "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nAuthorization: Bearer {}\r\nMcp-Protocol-Version: 2025-03-26\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                file.port,
                file.token.as_str(),
                body.len(),
            )
            .unwrap();
            let callback = callbacks
                .recv_timeout(Duration::from_secs(2))
                .expect("snapshot dispatched a WebKit evaluation");
            assert!(!callback.is_closed());

            webview_pending.send((callback, stream)).unwrap();
            let receipt = handle.shutdown().unwrap();
            assert_eq!(receipt.bound.port(), file.port);
            assert!(
                enumerate_discovery(&discovery_dir)
                    .unwrap()
                    .instances
                    .is_empty()
            );
            shutdown_finished.send(()).unwrap();
        });

        // Hold the socket and the WebKit completion through the join on
        // the owning thread. This bound precedes the evaluation deadline.
        let (_callback, _stream) = pending
            .recv_timeout(Duration::from_secs(5))
            .expect("snapshot reached WebKit");
        finished
            .recv_timeout(Duration::from_secs(2))
            .expect("shutdown waited for WebKit");
        app_thread.join().unwrap();
    }

    #[tokio::test]
    async fn snapshot_times_out_while_webkit_still_holds_the_callback() {
        let (app, callbacks) = app_with_held_dispatch();
        let handler = TauriControlHandler::new(
            app.handle().clone(),
            Arc::new(NoCommandBridge),
            Default::default(),
        );
        let result = tokio::time::timeout(
            Duration::from_secs(8),
            handler.snapshot(SnapshotRequest {
                window: None,
                webview: None,
            }),
        )
        .await
        .expect("snapshot has its own evaluation deadline");
        assert!(matches!(
            result,
            Err(ToolError::EvaluationFailed { message }) if message.contains("timed out")
        ));
        assert!(callbacks.try_recv().unwrap().is_closed());
    }
}
