use std::path::PathBuf;

use longhorn_core::{PhysicalPoint, PhysicalRect, PhysicalSize};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize as TauriPhysicalSize, Position, Rect,
    Runtime, Size, WebviewUrl,
    webview::{DownloadEvent, NewWindowResponse, PageLoadEvent, WebviewBuilder},
};

use crate::{
    ChildViewError, ChildViewNativeOperation, ChildViewPolicyEvent, ChildViewRuntime,
    ChildViewRuntimeEvent, ChildViewRuntimeEventKind, ChildViewSource, RuntimeAttachRequest,
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
                operation: ChildViewNativeOperation::Attach,
                detail: "mapped host window is unavailable".to_string(),
            })?;

        let navigation_spec = request.spec.clone();
        let load_callback = request.callback;
        let load_generation = request.generation;
        let load_island = request.spec.island_id().clone();
        let load_label = request.spec.child_label().clone();
        let load_policy = request.spec.policy_hooks().clone();
        let popup_policy = request.spec.policy_hooks().clone();
        let download_policy = request.spec.policy_hooks().clone();
        // The bounded source maps exactly one-to-one onto Tauri's own
        // construction surface: remote documents stay `External`, bundled
        // documents become the application's asset protocol through
        // `App`. No other URL form is ever constructed here.
        let webview_url = match request.spec.source() {
            ChildViewSource::External(url) => WebviewUrl::External(url.clone()),
            ChildViewSource::Bundled(document) => WebviewUrl::App(PathBuf::from(document.as_str())),
        };
        let builder = WebviewBuilder::new(request.spec.child_label().as_str(), webview_url)
            .focused(false)
            .disable_drag_drop_handler()
            .on_navigation(move |candidate| navigation_spec.allows_navigation(candidate))
            .on_new_window(move |url, _| {
                popup_policy.emit(ChildViewPolicyEvent::PopupDenied { url });
                NewWindowResponse::Deny
            })
            .on_download(move |_, event| {
                if let DownloadEvent::Requested { url, .. } = event {
                    download_policy.emit(ChildViewPolicyEvent::DownloadDenied { url });
                }
                false
            })
            .on_page_load(move |_, payload| {
                let kind = match payload.event() {
                    PageLoadEvent::Started => {
                        load_policy.emit(ChildViewPolicyEvent::PageLoadStarted {
                            url: payload.url().clone(),
                        });
                        ChildViewRuntimeEventKind::PageLoadStarted
                    }
                    PageLoadEvent::Finished => {
                        load_policy.emit(ChildViewPolicyEvent::PageLoadFinished {
                            url: payload.url().clone(),
                        });
                        ChildViewRuntimeEventKind::PageLoadFinished
                    }
                };
                load_callback(ChildViewRuntimeEvent {
                    island_id: load_island.clone(),
                    generation: load_generation,
                    child_label: load_label.clone(),
                    kind,
                });
            });

        let builder = if let Some(script) = request.spec.policy_hooks().initialization_script() {
            builder.initialization_script(script)
        } else {
            builder
        };

        #[cfg(target_os = "macos")]
        let builder = {
            let title_policy = request.spec.policy_hooks().clone();
            builder.on_document_title_changed(move |_, title| {
                title_policy.emit(ChildViewPolicyEvent::DocumentTitleChanged { title });
            })
        };

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
            .map_err(|error| native_error(ChildViewNativeOperation::Attach, error))?;
        hide_new_child(|| webview.hide(), || webview.close())?;
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
            .map_err(|error| native_error(ChildViewNativeOperation::Bounds, error))
    }

    fn show(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        handle
            .webview
            .show()
            .map_err(|error| native_error(ChildViewNativeOperation::Show, error))
    }

    fn hide(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        handle
            .webview
            .hide()
            .map_err(|error| native_error(ChildViewNativeOperation::Hide, error))
    }

    fn focus(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        handle
            .webview
            .set_focus()
            .map_err(|error| native_error(ChildViewNativeOperation::Focus, error))
    }

    fn current_url(&self, handle: &Self::Handle) -> Result<tauri::Url, ChildViewError> {
        handle
            .webview
            .url()
            .map_err(|error| native_error(ChildViewNativeOperation::CurrentUrl, error))
    }

    fn navigate(&self, handle: &Self::Handle, url: tauri::Url) -> Result<(), ChildViewError> {
        handle
            .webview
            .navigate(url)
            .map_err(|error| native_error(ChildViewNativeOperation::Navigate, error))
    }

    fn close(&self, handle: &Self::Handle) -> Result<(), ChildViewError> {
        handle
            .webview
            .close()
            .map_err(|error| native_error(ChildViewNativeOperation::Close, error))
    }

    fn bounds(&self, handle: &Self::Handle) -> Result<PhysicalRect, ChildViewError> {
        let position = handle
            .webview
            .position()
            .map_err(|error| native_error(ChildViewNativeOperation::Observe, error))?;
        let size = handle
            .webview
            .size()
            .map_err(|error| native_error(ChildViewNativeOperation::Observe, error))?;
        Ok(PhysicalRect::new(
            PhysicalPoint::new(position.x, position.y),
            PhysicalSize::new(size.width, size.height),
        ))
    }
}

fn hide_new_child(
    hide: impl FnOnce() -> tauri::Result<()>,
    close: impl FnOnce() -> tauri::Result<()>,
) -> Result<(), ChildViewError> {
    if let Err(error) = hide() {
        if let Err(close_error) = close() {
            longhorn_core::report_best_effort_failure(
                "native-content.child-view.close",
                format_args!("{close_error:?}"),
            );
        }
        return Err(native_error(ChildViewNativeOperation::Hide, error));
    }
    Ok(())
}

fn native_error(operation: ChildViewNativeOperation, error: tauri::Error) -> ChildViewError {
    ChildViewError::Native {
        operation,
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{ChildViewError, ChildViewNativeOperation, hide_new_child};

    #[test]
    fn failed_initial_hide_closes_child_and_preserves_hide_failure() {
        let mut closes = 0;
        let result = hide_new_child(
            || Err(tauri::Error::Io(std::io::Error::other("hide failed"))),
            || {
                closes += 1;
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(ChildViewError::Native {
                operation: ChildViewNativeOperation::Hide,
                ..
            })
        ));
        assert_eq!(closes, 1);
        hide_new_child(
            || Ok(()),
            || panic!("successful hide must retain the child"),
        )
        .unwrap();
    }
}
