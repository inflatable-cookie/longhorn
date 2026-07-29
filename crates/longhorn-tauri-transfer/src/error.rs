use std::{error::Error, fmt};

use longhorn_windowing::HostWindowHandle;

/// Failure while reading current managed Tauri window authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferRuntimeError {
    /// The shared managed-window host could not provide current windows.
    WindowHost(String),
    /// Native Tauri geometry probing failed.
    Probe(String),
    /// The injected desktop mapper rejected current geometry.
    Mapping(String),
    /// The invoking Tauri label is not in the managed-window registry.
    UnmanagedCaller(HostWindowHandle),
    /// A managed transport handle has no stable `WindowId`.
    MissingWindowId(HostWindowHandle),
    /// Current readback repeated one transport handle.
    DuplicateTransportHandle(HostWindowHandle),
    /// Current readback repeated one stable window identity.
    DuplicateWindowId(longhorn_core::WindowId),
    /// The mapper omitted one managed transport handle.
    MissingMappedWindow(HostWindowHandle),
    /// A native window property call failed.
    HostCall {
        /// Managed transport handle.
        handle: HostWindowHandle,
        /// Failed operation.
        operation: &'static str,
        /// Tauri error text.
        detail: String,
    },
    /// Checked physical-to-logical content-origin projection overflowed.
    ContentOriginOverflow(HostWindowHandle),
}

impl fmt::Display for TransferRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WindowHost(detail) => write!(formatter, "managed window host: {detail}"),
            Self::Probe(detail) => write!(formatter, "managed window probe: {detail}"),
            Self::Mapping(detail) => write!(formatter, "desktop mapping: {detail}"),
            Self::UnmanagedCaller(handle) => {
                write!(formatter, "caller {handle} is not a managed window")
            }
            Self::MissingWindowId(handle) => {
                write!(formatter, "managed window {handle} has no stable identity")
            }
            Self::DuplicateTransportHandle(handle) => {
                write!(formatter, "duplicate managed transport handle {handle}")
            }
            Self::DuplicateWindowId(window_id) => {
                write!(formatter, "duplicate managed window identity {window_id}")
            }
            Self::MissingMappedWindow(handle) => {
                write!(formatter, "desktop mapper omitted managed window {handle}")
            }
            Self::HostCall {
                handle,
                operation,
                detail,
            } => write!(formatter, "{operation} failed for {handle}: {detail}"),
            Self::ContentOriginOverflow(handle) => {
                write!(formatter, "content origin overflowed for {handle}")
            }
        }
    }
}

impl Error for TransferRuntimeError {}

/// Failure outside the renderer-visible transfer domain response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferHandlerError {
    /// Current managed-window readback failed.
    Runtime(TransferRuntimeError),
    /// Renderer-local geometry could not be projected safely.
    Projection(TransferProjectionError),
    /// Shared handler state was poisoned.
    StateUnavailable,
    /// The assembly was already torn down.
    Inactive,
    /// A process-local client id or epoch counter overflowed.
    IdentityExhausted,
    /// Host-issued client identity construction failed unexpectedly.
    InvalidIssuedClientId,
    /// Binding new host-issued client authority failed.
    ClientBinding(longhorn_transfer::TransferErrorCode, String),
}

impl fmt::Display for TransferHandlerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => error.fmt(formatter),
            Self::Projection(error) => error.fmt(formatter),
            Self::StateUnavailable => formatter.write_str("transfer handler state is unavailable"),
            Self::Inactive => formatter.write_str("transfer handler is inactive"),
            Self::IdentityExhausted => {
                formatter.write_str("transfer client identity space is exhausted")
            }
            Self::InvalidIssuedClientId => {
                formatter.write_str("host issued an invalid transfer client id")
            }
            Self::ClientBinding(code, detail) => {
                write!(formatter, "client binding {code:?}: {detail}")
            }
        }
    }
}

impl Error for TransferHandlerError {}

impl From<TransferRuntimeError> for TransferHandlerError {
    fn from(error: TransferRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<TransferProjectionError> for TransferHandlerError {
    fn from(error: TransferProjectionError) -> Self {
        Self::Projection(error)
    }
}

/// Failure while projecting renderer-local CSS geometry into screen DIPs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferProjectionError {
    /// Client coordinate arithmetic produced a non-finite value.
    NonFinite,
    /// A rounded coordinate or extent cannot be represented by Longhorn geometry.
    Overflow,
    /// A projected point falls outside the caller's current content bounds.
    PointOutsideContent,
    /// A projected rectangle has no positive extent.
    EmptyRectangle,
    /// A projected rectangle falls outside the caller's current content bounds.
    RectangleOutsideContent,
}

impl fmt::Display for TransferProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => formatter.write_str("client geometry arithmetic is non-finite"),
            Self::Overflow => formatter.write_str("client geometry projection overflowed"),
            Self::PointOutsideContent => {
                formatter.write_str("client point is outside current caller content")
            }
            Self::EmptyRectangle => {
                formatter.write_str("client rectangle must have positive projected extent")
            }
            Self::RectangleOutsideContent => {
                formatter.write_str("client rectangle is outside current caller content")
            }
        }
    }
}

impl Error for TransferProjectionError {}
