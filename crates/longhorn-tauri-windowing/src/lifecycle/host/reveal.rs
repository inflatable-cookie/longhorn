use longhorn_core::WindowId;
use tauri::Runtime;

use super::TauriWindowLifecycleHost;
use crate::{
    ApplyReadback, TauriApplyReceipt, TauriWindowLifecycleError, WindowRevealReceipt,
    WindowRevealStatus,
};

impl<R: Runtime> TauriWindowLifecycleHost<R> {
    /// Marks the renderer protocol ready and advances the two-signal reveal gate.
    pub fn mark_page_ready(
        &self,
        window_id: &WindowId,
    ) -> Result<WindowRevealReceipt, TauriWindowLifecycleError> {
        self.ensure_active()?;
        {
            let mut windows = self.lock_windows()?;
            let installed = windows.get_mut(window_id).ok_or_else(|| {
                TauriWindowLifecycleError::UnknownWindow {
                    window_id: window_id.clone(),
                }
            })?;
            installed.page_ready = true;
        }
        self.advance_reveal(window_id)
    }

    /// Marks converged hidden placement for successful move/resize attempts.
    pub fn mark_apply_converged(
        &self,
        receipt: &TauriApplyReceipt,
    ) -> Result<Vec<WindowRevealReceipt>, TauriWindowLifecycleError> {
        self.ensure_active()?;
        if !receipt.is_converged() {
            return Ok(Vec::new());
        }
        let ApplyReadback::Complete { observation, .. } = receipt.readback() else {
            return Ok(Vec::new());
        };
        let ids: Vec<WindowId> = observation
            .windows()
            .iter()
            .filter_map(|window| window.window_id().cloned())
            .collect();
        {
            let mut windows = self.lock_windows()?;
            for window_id in &ids {
                if let Some(installed) = windows.get_mut(window_id) {
                    installed.placement_ready = true;
                }
            }
        }
        ids.iter().map(|id| self.advance_reveal(id)).collect()
    }

    fn advance_reveal(
        &self,
        window_id: &WindowId,
    ) -> Result<WindowRevealReceipt, TauriWindowLifecycleError> {
        let window = {
            let mut windows = self.lock_windows()?;
            let installed = windows.get_mut(window_id).ok_or_else(|| {
                TauriWindowLifecycleError::UnknownWindow {
                    window_id: window_id.clone(),
                }
            })?;
            if installed.revealed {
                return Ok(WindowRevealReceipt::new(
                    window_id.clone(),
                    WindowRevealStatus::AlreadyRevealed,
                ));
            }
            if !installed.page_ready || !installed.placement_ready {
                return Ok(WindowRevealReceipt::new(
                    window_id.clone(),
                    WindowRevealStatus::Waiting {
                        page_ready: installed.page_ready,
                        placement_ready: installed.placement_ready,
                    },
                ));
            }
            if installed.reveal_started {
                return Ok(WindowRevealReceipt::new(
                    window_id.clone(),
                    WindowRevealStatus::Waiting {
                        page_ready: true,
                        placement_ready: true,
                    },
                ));
            }
            installed.reveal_started = true;
            installed.window.clone()
        };
        let result = self.services.reveal.reveal(&window);
        let mut windows = self.lock_windows()?;
        if let Some(installed) = windows.get_mut(window_id) {
            installed.reveal_started = false;
            if result.is_ok() {
                installed.revealed = true;
            }
        }
        Ok(WindowRevealReceipt::new(
            window_id.clone(),
            match result {
                Ok(()) => WindowRevealStatus::Revealed,
                Err(detail) => WindowRevealStatus::Failed { detail },
            },
        ))
    }
}
