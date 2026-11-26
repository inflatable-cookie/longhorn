use longhorn_agent_control::{ActionReceipt, ResizeWindowRequest, ToolError, WindowInfo};
use longhorn_core::{ClientSize, WindowId};
use tauri::{LogicalSize, PhysicalSize, Runtime, Size, Window};

pub(super) trait WindowOperations {
    fn title(&self) -> tauri::Result<String>;
    fn scale_factor(&self) -> tauri::Result<f64>;
    fn inner_size(&self) -> tauri::Result<PhysicalSize<u32>>;
    fn is_focused(&self) -> tauri::Result<bool>;
    fn set_logical_size(&self, size: LogicalSize<f64>) -> tauri::Result<()>;
}

impl<R: Runtime> WindowOperations for Window<R> {
    fn title(&self) -> tauri::Result<String> {
        Window::title(self)
    }

    fn scale_factor(&self) -> tauri::Result<f64> {
        Window::scale_factor(self)
    }

    fn inner_size(&self) -> tauri::Result<PhysicalSize<u32>> {
        Window::inner_size(self)
    }

    fn is_focused(&self) -> tauri::Result<bool> {
        Window::is_focused(self)
    }

    fn set_logical_size(&self, size: LogicalSize<f64>) -> tauri::Result<()> {
        self.set_size(Size::Logical(size))
    }
}

fn read_failed(label: &str, property: &str, error: tauri::Error) -> ToolError {
    ToolError::HostFailure {
        message: format!("window {label:?} {property} read failed: {error}"),
    }
}

pub(super) fn window_focused(
    label: &str,
    window: &impl WindowOperations,
) -> Result<bool, ToolError> {
    window
        .is_focused()
        .map_err(|error| read_failed(label, "focus", error))
}

pub(super) fn window_info(
    label: &str,
    window: &impl WindowOperations,
) -> Result<WindowInfo, ToolError> {
    let id = WindowId::new(label.to_owned()).map_err(|error| ToolError::HostFailure {
        message: format!("window label {label:?} is not a valid window id: {error}"),
    })?;
    let title = window
        .title()
        .map_err(|error| read_failed(label, "title", error))?;
    let scale = window
        .scale_factor()
        .map_err(|error| read_failed(label, "scale factor", error))?;
    let physical = window
        .inner_size()
        .map_err(|error| read_failed(label, "size", error))?;
    let logical = physical.to_logical::<f64>(scale);
    let size =
        ClientSize::new(logical.width, logical.height).map_err(|error| ToolError::HostFailure {
            message: format!("window {label:?} reported an invalid client size: {error}"),
        })?;
    Ok(WindowInfo {
        window: id,
        title,
        size,
        focused: window_focused(label, window)?,
    })
}

pub(super) fn resize_window(
    request: &ResizeWindowRequest,
    window: &impl WindowOperations,
) -> Result<ActionReceipt, ToolError> {
    window
        .set_logical_size(LogicalSize::new(
            request.size.width().get(),
            request.size.height().get(),
        ))
        .map_err(|error| ToolError::HostFailure {
            message: format!("window {:?} resize failed: {error}", request.window),
        })?;
    Ok(ActionReceipt {})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum FailingOperation {
        Title,
        ScaleFactor,
        InnerSize,
        Focus,
        Resize,
    }

    impl FailingOperation {
        fn result<T>(&self, operation: Self, value: T) -> tauri::Result<T> {
            if *self == operation {
                Err(std::io::Error::other("host rejected operation").into())
            } else {
                Ok(value)
            }
        }
    }

    impl WindowOperations for FailingOperation {
        fn title(&self) -> tauri::Result<String> {
            self.result(Self::Title, "Main window".to_owned())
        }

        fn scale_factor(&self) -> tauri::Result<f64> {
            self.result(Self::ScaleFactor, 2.0)
        }

        fn inner_size(&self) -> tauri::Result<PhysicalSize<u32>> {
            self.result(Self::InnerSize, PhysicalSize::new(1600, 1200))
        }

        fn is_focused(&self) -> tauri::Result<bool> {
            self.result(Self::Focus, false)
        }

        fn set_logical_size(&self, size: LogicalSize<f64>) -> tauri::Result<()> {
            assert_eq!(size, LogicalSize::new(800.0, 600.0));
            self.result(Self::Resize, ())
        }
    }

    #[test]
    fn each_window_read_reports_host_failure_with_its_operation() {
        for (operation, property) in [
            (FailingOperation::Title, "title"),
            (FailingOperation::ScaleFactor, "scale factor"),
            (FailingOperation::InnerSize, "size"),
            (FailingOperation::Focus, "focus"),
        ] {
            assert_eq!(
                window_info("main", &operation),
                Err(ToolError::HostFailure {
                    message: format!(
                        "window \"main\" {property} read failed: host rejected operation"
                    ),
                }),
                "{operation:?}"
            );
        }
    }

    #[test]
    fn resize_reports_host_failure() {
        let request = ResizeWindowRequest {
            window: WindowId::new("main").unwrap(),
            size: ClientSize::new(800.0, 600.0).unwrap(),
        };
        assert_eq!(
            resize_window(&request, &FailingOperation::Resize),
            Err(ToolError::HostFailure {
                message: format!(
                    "window {:?} resize failed: host rejected operation",
                    request.window
                ),
            }),
        );
    }
}
