use std::collections::BTreeSet;

use longhorn_core::{DropZoneId, ScreenRect, WindowId};

use crate::{
    LeaseGeneration, LeasePublication, MonotonicClock, TransferError, TransferErrorCode,
    TransferInstant,
};

use super::{LeasePublicationReceipt, LeaseRecord, TransferCoordinator};

impl TransferCoordinator {
    /// Atomically publishes one complete current replacement lease.
    pub fn publish_lease(
        &mut self,
        clock: &impl MonotonicClock,
        publication: LeasePublication,
    ) -> Result<LeasePublicationReceipt, TransferError> {
        let now = self.observe_now(clock)?;
        let expires_at = self.checked_expiry(
            now,
            publication.lifetime(),
            self.limits.maximum_lease_lifetime(),
            "lease",
        )?;
        self.require_client(
            publication.window_id(),
            publication.client_id(),
            publication.client_epoch(),
        )?;
        validate_publication(self.limits, &publication)?;
        if !self.leases.contains_key(publication.window_id())
            && self.leases.len() >= self.limits.maximum_leases()
        {
            return Err(TransferError::new(
                TransferErrorCode::LeaseCapacity,
                "complete drop-zone lease registry is full",
            ));
        }

        if let Some(current) = self.leases.get(publication.window_id()) {
            if publication.generation() <= current.generation {
                return Err(TransferError::new(
                    TransferErrorCode::StaleLeaseGeneration,
                    format!(
                        "lease generation {} does not advance current generation {} for {}",
                        publication.generation().get(),
                        current.generation.get(),
                        publication.window_id()
                    ),
                ));
            }
        }

        let receipt = LeasePublicationReceipt {
            generation: publication.generation(),
            expires_at,
            zone_count: publication.zones().len(),
        };
        self.leases.insert(
            publication.window_id().clone(),
            LeaseRecord {
                client_id: publication.client_id().clone(),
                client_epoch: publication.client_epoch(),
                generation: publication.generation(),
                expires_at,
                window_outer_bounds: publication.window_outer_bounds(),
                zones: publication.zones().to_vec(),
            },
        );
        Ok(receipt)
    }

    /// Returns the current installed generation for one window.
    #[must_use]
    pub fn current_lease_generation(&self, window_id: &WindowId) -> Option<LeaseGeneration> {
        self.leases.get(window_id).map(|lease| lease.generation)
    }
}

fn validate_publication(
    limits: crate::TransferLimits,
    publication: &LeasePublication,
) -> Result<(), TransferError> {
    validate_rect(publication.window_outer_bounds(), "window outer bounds")?;
    if publication.zones().len() > limits.maximum_zones_per_lease() {
        return Err(invalid_lease(format!(
            "lease has {} zones; maximum is {}",
            publication.zones().len(),
            limits.maximum_zones_per_lease()
        )));
    }

    let mut ids = BTreeSet::<DropZoneId>::new();
    for zone in publication.zones() {
        if !ids.insert(zone.id().clone()) {
            return Err(invalid_lease(format!(
                "lease contains duplicate zone {}",
                zone.id()
            )));
        }
        validate_rect(zone.bounds(), "drop-zone bounds")?;
        if !publication
            .window_outer_bounds()
            .contains_rect(&zone.bounds())
        {
            return Err(invalid_lease(format!(
                "drop zone {} falls outside current window bounds",
                zone.id()
            )));
        }
        if zone
            .insertion_position()
            .is_some_and(|position| position.get() > limits.maximum_insertion_position())
        {
            return Err(invalid_lease(format!(
                "drop zone {} insertion position exceeds {}",
                zone.id(),
                limits.maximum_insertion_position()
            )));
        }
        if zone.accepted_capability() != zone.target().capability() {
            return Err(invalid_lease(format!(
                "drop zone {} capability does not match its target binding",
                zone.id()
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_rect(rect: ScreenRect, label: &str) -> Result<(), TransferError> {
    if rect.size().is_empty() {
        return Err(invalid_lease(format!("{label} must have positive extent")));
    }
    let width = i32::try_from(rect.size().width())
        .map_err(|_| invalid_lease(format!("{label} width exceeds screen coordinates")))?;
    let height = i32::try_from(rect.size().height())
        .map_err(|_| invalid_lease(format!("{label} height exceeds screen coordinates")))?;
    rect.origin()
        .x()
        .get()
        .checked_add(width)
        .ok_or_else(|| invalid_lease(format!("{label} right edge overflows screen coordinates")))?;
    rect.origin().y().get().checked_add(height).ok_or_else(|| {
        invalid_lease(format!("{label} bottom edge overflows screen coordinates"))
    })?;
    Ok(())
}

fn invalid_lease(detail: impl Into<String>) -> TransferError {
    TransferError::new(TransferErrorCode::InvalidLease, detail)
}

pub(super) fn lease_expired(window_id: &WindowId, now: TransferInstant) -> TransferError {
    TransferError::new(
        TransferErrorCode::LeaseExpired,
        format!(
            "drop-zone lease for {window_id} is expired at {}",
            now.get()
        ),
    )
}
