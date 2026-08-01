use longhorn_core::{PhysicalPoint, PhysicalRect, PhysicalSize};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize as TauriPhysicalSize, Position, Rect,
    Runtime, Size, WebviewUrl,
    webview::{DownloadEvent, NewWindowResponse, PageLoadEvent, WebviewBuilder},
};

use crate::{
    ChildWebviewError, ChildWebviewRuntime, DownloadPolicy, PopupPolicy, RuntimeAttachRequest,
    RuntimeEvent, RuntimeEventKind,
};

/// Tauri 2 implementation that confines the unstable child-webview API.
pub struct TauriChildWebviewRuntime<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> Clone for TauriChildWebviewRuntime<R> {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
        }
    }
}

impl<R: Runtime> TauriChildWebviewRuntime<R> {
    /// Creates a runtime port from the consumer's Tauri app handle.
    #[must_use]
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> ChildWebviewRuntime for TauriChildWebviewRuntime<R> {
    type Handle = tauri::Webview<R>;

    fn attach(&self, request: RuntimeAttachRequest) -> Result<Self::Handle, ChildWebviewError> {
        #[cfg(not(target_os = "macos"))]
        if request.spec.data_store_identifier().is_some() {
            return Err(ChildWebviewError::UnsupportedDataStorePolicy);
        }

        let host = self
            .app
            .get_window(request.spec.host_window_label().as_str())
            .ok_or_else(|| ChildWebviewError::Native {
                operation: "attach",
                detail: "mapped host window is unavailable".to_string(),
            })?;
        let callback = request.callback.clone();
        let navigation_callback = callback.clone();
        let navigation_spec = request.spec.clone();
        let navigation_generation = request.generation;
        let navigation_island = request.spec.island_id().clone();
        let navigation_label = request.spec.webview_label().as_str().to_string();
        let popup_callback = callback.clone();
        let popup_generation = request.generation;
        let popup_island = request.spec.island_id().clone();
        let popup_label = request.spec.webview_label().as_str().to_string();
        let popup_policy = request.spec.popup_policy();
        let download_callback = callback.clone();
        let download_generation = request.generation;
        let download_island = request.spec.island_id().clone();
        let download_label = request.spec.webview_label().as_str().to_string();
        let download_policy = request.spec.download_policy();
        let load_callback = callback;
        let load_generation = request.generation;
        let load_island = request.spec.island_id().clone();
        let load_label = request.spec.webview_label().as_str().to_string();

        let builder = WebviewBuilder::new(
            request.spec.webview_label().as_str(),
            WebviewUrl::External(request.spec.source().clone()),
        )
        .focused(false)
        .disable_drag_drop_handler()
        .on_navigation(move |url| {
            let allowed = navigation_spec.allows_navigation(url);
            navigation_callback(RuntimeEvent {
                island_id: navigation_island.clone(),
                generation: navigation_generation,
                webview_label: navigation_label.clone(),
                kind: RuntimeEventKind::Navigation {
                    url: url.to_string(),
                    allowed,
                },
            });
            allowed
        })
        .on_new_window(move |url, _| {
            debug_assert_eq!(popup_policy, PopupPolicy::Deny);
            popup_callback(RuntimeEvent {
                island_id: popup_island.clone(),
                generation: popup_generation,
                webview_label: popup_label.clone(),
                kind: RuntimeEventKind::PopupDenied {
                    url: url.to_string(),
                },
            });
            NewWindowResponse::Deny
        })
        .on_download(move |_, event| {
            debug_assert_eq!(download_policy, DownloadPolicy::Deny);
            if let DownloadEvent::Requested { url, .. } = event {
                download_callback(RuntimeEvent {
                    island_id: download_island.clone(),
                    generation: download_generation,
                    webview_label: download_label.clone(),
                    kind: RuntimeEventKind::DownloadDenied {
                        url: url.to_string(),
                    },
                });
            }
            false
        })
        .on_page_load(move |_, payload| {
            let kind = match payload.event() {
                PageLoadEvent::Started => RuntimeEventKind::PageLoadStarted {
                    url: payload.url().to_string(),
                },
                PageLoadEvent::Finished => RuntimeEventKind::PageLoadFinished {
                    url: payload.url().to_string(),
                },
            };
            load_callback(RuntimeEvent {
                island_id: load_island.clone(),
                generation: load_generation,
                webview_label: load_label.clone(),
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
            .map_err(|error| ChildWebviewError::Native {
                operation: "attach",
                detail: error.to_string(),
            })?;
        webview.hide().map_err(|error| ChildWebviewError::Native {
            operation: "hide",
            detail: error.to_string(),
        })?;
        Ok(webview)
    }

    fn set_bounds(
        &self,
        handle: &Self::Handle,
        bounds: PhysicalRect,
    ) -> Result<(), ChildWebviewError> {
        handle
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
            .map_err(|error| ChildWebviewError::Native {
                operation: "bounds",
                detail: error.to_string(),
            })
    }

    fn show(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError> {
        handle.show().map_err(|error| ChildWebviewError::Native {
            operation: "show",
            detail: error.to_string(),
        })
    }

    fn hide(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError> {
        handle.hide().map_err(|error| ChildWebviewError::Native {
            operation: "hide",
            detail: error.to_string(),
        })
    }

    fn focus(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError> {
        handle
            .set_focus()
            .map_err(|error| ChildWebviewError::Native {
                operation: "focus",
                detail: error.to_string(),
            })
    }

    fn close(&self, handle: &Self::Handle) -> Result<(), ChildWebviewError> {
        handle.close().map_err(|error| ChildWebviewError::Native {
            operation: "close",
            detail: error.to_string(),
        })
    }

    fn bounds(&self, handle: &Self::Handle) -> Result<PhysicalRect, ChildWebviewError> {
        let position = handle
            .position()
            .map_err(|error| ChildWebviewError::Native {
                operation: "observe",
                detail: error.to_string(),
            })?;
        let size = handle.size().map_err(|error| ChildWebviewError::Native {
            operation: "observe",
            detail: error.to_string(),
        })?;
        Ok(PhysicalRect::new(
            PhysicalPoint::new(position.x, position.y),
            PhysicalSize::new(size.width, size.height),
        ))
    }

    fn evaluate(&self, handle: &Self::Handle, script: &str) -> Result<(), ChildWebviewError> {
        handle
            .eval(script)
            .map_err(|error| ChildWebviewError::Native {
                operation: "evaluate",
                detail: error.to_string(),
            })
    }
}
