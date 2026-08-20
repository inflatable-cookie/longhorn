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

use std::sync::{Arc, Mutex};

use block2::RcBlock;
use objc2::{AllocAnyThread, msg_send, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSCalibratedRGBColorSpace, NSCompositingOperation,
    NSGraphicsContext, NSImage,
};
use objc2_foundation::{NSArray, NSData, NSError, NSDictionary, NSNumber, NSPoint, NSRect, NSSize, NSString};
use objc2_web_kit::WKWebView;
use serde_json::Value;
use tauri::{PhysicalSize, Runtime, Webview, Window};
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
    let inner = window
        .inner_size()
        .map_err(|error| capture_failed(format!("window size read failed: {error}")))?;
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
            let position = webview.position().map_err(|error| {
                capture_failed(format!("webview {label:?} position read failed: {error}"))
            })?;
            let size = webview.size().map_err(|error| {
                capture_failed(format!("webview {label:?} size read failed: {error}"))
            })?;
            (
                f64::from(position.x),
                f64::from(position.y),
                f64::from(size.width),
                f64::from(size.height),
            )
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
    composite(window, inner, surfaces).await
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
    let tx = Arc::new(Mutex::new(Some(tx)));
    webview_handle
        .with_webview(move |webview| {
            let tx = Arc::clone(&tx);
            let Some(webview) = retain_webview(&webview, &tx, capture_failed) else {
                return;
            };
            if webview.isHiddenOrHasHiddenAncestor() {
                send(&tx, Ok(None));
                return;
            }
            let order = z_order(&webview);
            let completion = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
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

/// Draws the surfaces into a physical-pixel bitmap of the window's inner
/// size and returns the PNG. Runs entirely on the main thread: AppKit
/// drawing is main-thread-only, and only PNG bytes cross threads.
async fn composite<R: Runtime>(
    window: &Window<R>,
    inner: PhysicalSize<u32>,
    surfaces: Vec<Surface>,
) -> Result<Vec<u8>, ToolError> {
    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));
    window
        .run_on_main_thread(move || {
            send(&tx, compose_png(inner, surfaces));
        })
        .map_err(|error| capture_failed(format!("main-thread dispatch failed: {error}")))?;
    rx.await
        .map_err(|_| capture_failed("the main thread dropped the composite result"))?
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

/// The vocabulary's closest fit for a capture-side failure: the surface
/// could not serve this request, with the reason in the message.
fn capture_failed(message: impl Into<String>) -> ToolError {
    ToolError::Unsupported {
        message: format!("screenshot capture failed: {}", message.into()),
    }
}
