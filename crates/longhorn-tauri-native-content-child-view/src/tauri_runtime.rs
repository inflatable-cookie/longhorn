use longhorn_core::{PhysicalPoint, PhysicalRect, PhysicalSize};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize as TauriPhysicalSize, Position, Rect,
    Runtime, Size, WebviewUrl,
    webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder},
};

use crate::{
    ChildViewError, ChildViewRuntime, ChildViewRuntimeEvent, ChildViewRuntimeEventKind,
    RuntimeAttachRequest,
};

/// Opaque retained child handle. The underlying Tauri webview stays private.
pub struct TauriChildViewHandle<R: Runtime> {
    webview: tauri::Webview<R>,
}

impl<R: Runtime> Clone for TauriChildViewHandle<R> {
    fn clone(&self) -> Self {
        Self {
            webview: self.webview.clone(),
        }
    }
}

/// Tauri 2 implementation that confines the unstable child-webview API.
pub struct TauriChildViewRuntime<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> Clone for TauriChildViewRuntime<R> {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
        }
    }
}

impl<R: Runtime> TauriChildViewRuntime<R> {
    /// Creates a runtime port from the consumer's Tauri app handle.
    #[must_use]
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> ChildViewRuntime for TauriChildViewRuntime<R> {
    type Handle = TauriChildViewHandle<R>;

    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, ChildViewError> {
        #[cfg(not(target_os = "macos"))]
        if request.spec.data_store_identifier().is_some() {
            return Err(ChildViewError::UnsupportedDataStorePolicy);
        }

        let host = self
            .app
            .get_window(request.spec.host_window_label().as_str())
            .ok_or_else(|| ChildViewError::Native {
                operation: "attach",
                detail: "mapped host window is unavailable".to_string(),
            })?;

        let navigation_spec = request.spec.clone();
        let load_callback = request.callback;
        let load_generation = request.generation;
        let load_island = request.spec.island_id().clone();
        let load_label = request.spec.child_label().clone();
        let builder = WebviewBuilder::new(
            request.spec.child_label().as_str(),
            WebviewUrl::External(request.spec.source().clone()),
        )
        .focused(false)
        .disable_drag_drop_handler()
        .on_navigation(move |candidate| navigation_spec.allows_navigation(candidate))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .on_download(|_, _| false)
        .on_page_load(move |_, payload| {
            let kind = match payload.event() {
                PageLoadEvent::Started => ChildViewRuntimeEventKind::PageLoadStarted,
                PageLoadEvent::Finished => ChildViewRuntimeEventKind::PageLoadFinished,
            };
            load_callback(ChildViewRuntimeEvent {
                island_id: load_island.clone(),
                generation: load_generation,
                child_label: load_label.clone(),
                kind,
            });
        });

        #[cfg(target_os = "macos")]
        let builder = if let Some(identifier) = request.spec.data_store_identifier() {
            builder.data_store_identifier(identifier)
        } else {
            builder
        };

        let webview = host
            .add_child(
                builder,
                PhysicalPosition::new(0, 0),
                TauriPhysicalSize::new(1, 1),
            )
            .map_err(|error| native_error("attach", error))?;
        webview
            .hide()
            .map_err(|error| native_error("hide", error))?;
        Ok(TauriChildViewHandle { webview })
    }

    fn set_bounds(
        &self,
        handle: &Self::Handle,
        bounds: PhysicalRect,
    ) -> Result<(), ChildViewError> {
        handle
            .webview
            .set_bounds(Rect {
                position: Position::Physical(PhysicalPosition::new(
                    bounds.origin().x().get(),
                    bounds.origin().y().get(),
                )),
                size: Size::Physical(TauriPhysicalSize::new(
                    bounds.size().width(),
                    bounds.size().height(),
                )),
            })
            .map_err(|error| native_error("bounds", error))
    }

    fn show(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        handle
            .webview
            .show()
            .map_err(|error| native_error("show", error))
    }

    fn hide(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        handle
            .webview
            .hide()
            .map_err(|error| native_error("hide", error))
    }

    fn focus(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        handle
            .webview
            .set_focus()
            .map_err(|error| native_error("focus", error))
    }

    fn close(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        handle
            .webview
            .close()
            .map_err(|error| native_error("close", error))
    }

    fn bounds(&self, handle: &Self::Handle) -> Result<PhysicalRect, ChildViewError> {
        let position = handle
            .webview
            .position()
            .map_err(|error| native_error("observe", error))?;
        let size = handle
            .webview
            .size()
            .map_err(|error| native_error("observe", error))?;
        Ok(PhysicalRect::new(
            PhysicalPoint::new(position.x, position.y),
            PhysicalSize::new(size.width, size.height),
        ))
    }
}

fn native_error(operation: &'static str, error: tauri::Error) -> ChildViewError {
    ChildViewError::Native {
        operation,
        detail: error.to_string(),
    }
}
