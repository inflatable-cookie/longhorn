#[cfg(target_os = "macos")]
mod implementation {
    use longhorn_core::{PhysicalPoint, PhysicalRect, PhysicalSize};
    use objc2::rc::Retained;
    use objc2_app_kit::{
        NSAutoresizingMaskOptions, NSColor, NSView, NSWindow, NSWindowOrderingMode,
    };
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_foundation::MainThreadMarker;
    use tauri::WebviewWindow;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) struct NativeToken {
        root: usize,
        output: usize,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct NativeEvidence {
        pub(crate) storage_bounds: PhysicalRect,
        pub(crate) attached: bool,
    }

    pub(crate) fn attach(
        window: &WebviewWindow,
        scale: f64,
    ) -> Result<(NativeToken, NativeEvidence), String> {
        let mtm =
            MainThreadMarker::new().ok_or_else(|| "attach is not on main thread".to_string())?;
        let ns_window = window
            .ns_window()
            .map_err(|error| format!("could not resolve Tauri NSWindow: {error}"))?;
        let ns_window: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };
        let content = ns_window
            .contentView()
            .ok_or_else(|| "native window has no content view".to_string())?;
        let bounds = content.bounds();
        let root: Retained<NSView> = NSView::initWithFrame(mtm.alloc(), bounds);
        root.setWantsLayer(true);
        root.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        let output: Retained<NSView> = NSView::initWithFrame(mtm.alloc(), CGRect::ZERO);
        output.setWantsLayer(true);
        let layer = output
            .layer()
            .ok_or_else(|| "renderer output view has no layer".to_string())?;
        let color = NSColor::colorWithSRGBRed_green_blue_alpha(0.11, 0.66, 0.74, 1.0);
        layer.setBackgroundColor(Some(&color.CGColor()));
        root.addSubview(&output);
        content.addSubview_positioned_relativeTo(&root, NSWindowOrderingMode::Below, None);

        let output_ptr = Retained::as_ptr(&output) as usize;
        let root_ptr = Retained::into_raw(root) as usize;
        let token = NativeToken {
            root: root_ptr,
            output: output_ptr,
        };
        set_clip(
            token,
            PhysicalRect::new(PhysicalPoint::new(0, 0), PhysicalSize::new(0, 0)),
            scale,
            false,
        )?;
        Ok((
            token,
            NativeEvidence {
                storage_bounds: physical_bounds(bounds, scale)?,
                attached: is_attached(token),
            },
        ))
    }

    pub(crate) fn refresh(
        window: &WebviewWindow,
        token: NativeToken,
        clip: PhysicalRect,
        scale: f64,
        presentation_enabled: bool,
    ) -> Result<NativeEvidence, String> {
        let ns_window = window
            .ns_window()
            .map_err(|error| format!("could not resolve Tauri NSWindow: {error}"))?;
        let ns_window: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };
        let content = ns_window
            .contentView()
            .ok_or_else(|| "native window has no content view".to_string())?;
        let bounds = content.bounds();
        view(token.root).setFrame(bounds);
        set_clip(token, clip, scale, presentation_enabled)?;
        Ok(NativeEvidence {
            storage_bounds: physical_bounds(bounds, scale)?,
            attached: is_attached(token),
        })
    }

    pub(crate) fn set_clip(
        token: NativeToken,
        clip: PhysicalRect,
        scale: f64,
        presentation_enabled: bool,
    ) -> Result<(), String> {
        require_scale(scale)?;
        let root_bounds = view(token.root).bounds();
        let x = f64::from(clip.origin().x().get()) / scale;
        let width = f64::from(clip.size().width()) / scale;
        let height = f64::from(clip.size().height()) / scale;
        let top = f64::from(clip.origin().y().get()) / scale;
        let y = root_bounds.size.height - top - height;
        let output = view(token.output);
        output.setFrame(CGRect::new(CGPoint::new(x, y), CGSize::new(width, height)));
        output.setHidden(!presentation_enabled || clip.size().is_empty());
        Ok(())
    }

    pub(crate) fn detach(token: NativeToken, release: bool) -> Result<(), String> {
        if !release {
            return Ok(());
        }
        let root = unsafe { Retained::from_raw(token.root as *mut NSView) }
            .ok_or_else(|| "native root retain is missing".to_string())?;
        root.removeFromSuperview();
        drop(root);
        Ok(())
    }

    fn is_attached(token: NativeToken) -> bool {
        unsafe { view(token.root).superview().is_some() }
    }

    fn view(pointer: usize) -> &'static NSView {
        unsafe { &*(pointer as *const NSView) }
    }

    fn physical_bounds(bounds: CGRect, scale: f64) -> Result<PhysicalRect, String> {
        require_scale(scale)?;
        let width = (bounds.size.width * scale).round();
        let height = (bounds.size.height * scale).round();
        if width < 0.0
            || height < 0.0
            || width > f64::from(u32::MAX)
            || height > f64::from(u32::MAX)
        {
            return Err("native content bounds exceed physical range".to_string());
        }
        Ok(PhysicalRect::new(
            PhysicalPoint::new(0, 0),
            PhysicalSize::new(width as u32, height as u32),
        ))
    }

    fn require_scale(scale: f64) -> Result<(), String> {
        if scale.is_finite() && scale > 0.0 {
            Ok(())
        } else {
            Err(format!("invalid native scale {scale}"))
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) use implementation::{NativeToken, attach, detach, refresh, set_clip};

#[cfg(not(target_os = "macos"))]
mod implementation {
    use longhorn_core::PhysicalRect;
    use tauri::WebviewWindow;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) struct NativeToken;

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct NativeEvidence {
        pub(crate) storage_bounds: PhysicalRect,
        pub(crate) attached: bool,
    }

    pub(crate) fn attach(
        _window: &WebviewWindow,
        _scale: f64,
    ) -> Result<(NativeToken, NativeEvidence), String> {
        Err(format!(
            "backing-surface proof is unsupported on {}",
            std::env::consts::OS
        ))
    }

    pub(crate) fn refresh(
        _window: &WebviewWindow,
        _token: NativeToken,
        _clip: PhysicalRect,
        _scale: f64,
        _presentation_enabled: bool,
    ) -> Result<NativeEvidence, String> {
        Err(format!(
            "backing-surface proof is unsupported on {}",
            std::env::consts::OS
        ))
    }

    pub(crate) fn set_clip(
        _token: NativeToken,
        _clip: PhysicalRect,
        _scale: f64,
        _presentation_enabled: bool,
    ) -> Result<(), String> {
        Err(format!(
            "backing-surface proof is unsupported on {}",
            std::env::consts::OS
        ))
    }

    pub(crate) fn detach(_token: NativeToken, _release: bool) -> Result<(), String> {
        Err(format!(
            "backing-surface proof is unsupported on {}",
            std::env::consts::OS
        ))
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) use implementation::{NativeToken, attach, detach, refresh, set_clip};
