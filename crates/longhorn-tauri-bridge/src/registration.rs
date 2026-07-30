use std::{collections::BTreeMap, sync::Mutex};

use longhorn_bridge::{
    BridgeCancellationReceipt, BridgeCancellationRequest, BridgeCommandEnvelope,
    BridgeCommandReply, BridgeQueryEnvelope, BridgeQueryReply, BridgeSnapshotEnvelope,
    DomainCapabilityDescriptor,
};
use longhorn_core::{BridgeCapabilityId, BridgeSessionId, DomainId};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{BridgeHostError, BridgeHostErrorCode};

const MAXIMUM_ROUTE_BYTES: usize = 128;

/// Typed query handler retained behind one registered opaque route.
pub trait BridgeQueryHandler<Request, Success, Detail>: Send {
    /// Executes one already-authorized typed query.
    fn handle(
        &mut self,
        request: BridgeQueryEnvelope<Request>,
    ) -> Result<BridgeQueryReply<Success, Detail>, BridgeHostError>;
}

impl<Request, Success, Detail, F> BridgeQueryHandler<Request, Success, Detail> for F
where
    F: FnMut(
            BridgeQueryEnvelope<Request>,
        ) -> Result<BridgeQueryReply<Success, Detail>, BridgeHostError>
        + Send,
{
    fn handle(
        &mut self,
        request: BridgeQueryEnvelope<Request>,
    ) -> Result<BridgeQueryReply<Success, Detail>, BridgeHostError> {
        self(request)
    }
}

/// Typed authoritative command handler retained behind one registered route.
pub trait BridgeCommandHandler<Request, Success, Detail>: Send {
    /// Executes one already-authorized typed command.
    fn handle(
        &mut self,
        request: BridgeCommandEnvelope<Request>,
    ) -> Result<BridgeCommandReply<Success, Detail>, BridgeHostError>;
}

impl<Request, Success, Detail, F> BridgeCommandHandler<Request, Success, Detail> for F
where
    F: FnMut(
            BridgeCommandEnvelope<Request>,
        ) -> Result<BridgeCommandReply<Success, Detail>, BridgeHostError>
        + Send,
{
    fn handle(
        &mut self,
        request: BridgeCommandEnvelope<Request>,
    ) -> Result<BridgeCommandReply<Success, Detail>, BridgeHostError> {
        self(request)
    }
}

/// Typed cancellation handler retained behind one registered route.
pub trait BridgeCancellationHandler<Detail>: Send {
    /// Executes one already-authorized cancellation request.
    fn handle(
        &mut self,
        request: BridgeCancellationRequest,
    ) -> Result<BridgeCancellationReceipt<Detail>, BridgeHostError>;
}

impl<Detail, F> BridgeCancellationHandler<Detail> for F
where
    F: FnMut(
            BridgeCancellationRequest,
        ) -> Result<BridgeCancellationReceipt<Detail>, BridgeHostError>
        + Send,
{
    fn handle(
        &mut self,
        request: BridgeCancellationRequest,
    ) -> Result<BridgeCancellationReceipt<Detail>, BridgeHostError> {
        self(request)
    }
}

/// Typed authoritative snapshot handler for one registered domain.
pub trait BridgeSnapshotHandler<Snapshot>: Send {
    /// Loads one current snapshot for an already-authorized session.
    fn snapshot(
        &mut self,
        session_id: &BridgeSessionId,
        domain_id: &DomainId,
    ) -> Result<BridgeSnapshotEnvelope<Snapshot>, BridgeHostError>;
}

impl<Snapshot, F> BridgeSnapshotHandler<Snapshot> for F
where
    F: FnMut(
            &BridgeSessionId,
            &DomainId,
        ) -> Result<BridgeSnapshotEnvelope<Snapshot>, BridgeHostError>
        + Send,
{
    fn snapshot(
        &mut self,
        session_id: &BridgeSessionId,
        domain_id: &DomainId,
    ) -> Result<BridgeSnapshotEnvelope<Snapshot>, BridgeHostError> {
        self(session_id, domain_id)
    }
}

type ErasedQuery = Box<
    dyn FnMut(BridgeQueryEnvelope<Value>) -> Result<BridgeQueryReply<Value, Value>, BridgeHostError>
        + Send,
>;
type ErasedCommand = Box<
    dyn FnMut(
            BridgeCommandEnvelope<Value>,
        ) -> Result<BridgeCommandReply<Value, Value>, BridgeHostError>
        + Send,
>;
type ErasedCancellation = Box<
    dyn FnMut(
            BridgeCancellationRequest,
        ) -> Result<BridgeCancellationReceipt<Value>, BridgeHostError>
        + Send,
>;
type ErasedSnapshot = Box<
    dyn FnMut(&BridgeSessionId, &DomainId) -> Result<BridgeSnapshotEnvelope<Value>, BridgeHostError>
        + Send,
>;

pub(crate) enum RegisteredHandler {
    Query(Mutex<ErasedQuery>),
    Command(Mutex<ErasedCommand>),
    Cancellation(Mutex<ErasedCancellation>),
}

pub(crate) struct RegisteredRoute {
    pub(crate) domain_id: DomainId,
    pub(crate) required_capability: BridgeCapabilityId,
    pub(crate) handler: RegisteredHandler,
}

pub(crate) struct RegisteredSnapshot {
    pub(crate) required_capability: BridgeCapabilityId,
    pub(crate) handler: Mutex<ErasedSnapshot>,
}

/// Immutable registered domain, route, and typed handler catalogue.
#[derive(Default)]
pub struct BridgeDomainRegistry {
    domains: BTreeMap<DomainId, DomainCapabilityDescriptor>,
    routes: BTreeMap<String, RegisteredRoute>,
    snapshots: BTreeMap<DomainId, RegisteredSnapshot>,
}

impl BridgeDomainRegistry {
    /// Constructs an empty registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
            routes: BTreeMap::new(),
            snapshots: BTreeMap::new(),
        }
    }

    /// Registers one domain capability declaration without granting authority.
    pub fn register_domain(
        &mut self,
        descriptor: DomainCapabilityDescriptor,
    ) -> Result<(), BridgeHostError> {
        let domain_id = descriptor.domain_id().clone();
        if self.domains.contains_key(&domain_id) {
            return Err(registration_error(format!(
                "duplicate bridge domain registration: {domain_id}"
            )));
        }
        self.domains.insert(domain_id, descriptor);
        Ok(())
    }

    /// Registers one typed query route.
    pub fn register_query<Request, Success, Detail, Handler>(
        &mut self,
        domain_id: DomainId,
        route: impl Into<String>,
        required_capability: BridgeCapabilityId,
        mut handler: Handler,
    ) -> Result<(), BridgeHostError>
    where
        Request: DeserializeOwned + 'static,
        Success: Serialize + 'static,
        Detail: Serialize + 'static,
        Handler: BridgeQueryHandler<Request, Success, Detail> + 'static,
    {
        let route = route.into();
        self.insert_route(
            route,
            RegisteredRoute {
                domain_id,
                required_capability,
                handler: RegisteredHandler::Query(Mutex::new(Box::new(move |request| {
                    let typed = transcode(request)?;
                    transcode(handler.handle(typed)?)
                }))),
            },
        )
    }

    /// Registers one typed authoritative command route.
    pub fn register_command<Request, Success, Detail, Handler>(
        &mut self,
        domain_id: DomainId,
        route: impl Into<String>,
        required_capability: BridgeCapabilityId,
        mut handler: Handler,
    ) -> Result<(), BridgeHostError>
    where
        Request: DeserializeOwned + 'static,
        Success: Serialize + 'static,
        Detail: Serialize + 'static,
        Handler: BridgeCommandHandler<Request, Success, Detail> + 'static,
    {
        let route = route.into();
        self.insert_route(
            route,
            RegisteredRoute {
                domain_id,
                required_capability,
                handler: RegisteredHandler::Command(Mutex::new(Box::new(move |request| {
                    let typed = transcode(request)?;
                    transcode(handler.handle(typed)?)
                }))),
            },
        )
    }

    /// Registers one typed cancellation route.
    pub fn register_cancellation<Detail, Handler>(
        &mut self,
        domain_id: DomainId,
        route: impl Into<String>,
        required_capability: BridgeCapabilityId,
        mut handler: Handler,
    ) -> Result<(), BridgeHostError>
    where
        Detail: Serialize + 'static,
        Handler: BridgeCancellationHandler<Detail> + 'static,
    {
        let route = route.into();
        self.insert_route(
            route,
            RegisteredRoute {
                domain_id,
                required_capability,
                handler: RegisteredHandler::Cancellation(Mutex::new(Box::new(move |request| {
                    transcode(handler.handle(request)?)
                }))),
            },
        )
    }

    /// Registers one typed authoritative snapshot provider for a domain.
    pub fn register_snapshot<Snapshot, Handler>(
        &mut self,
        domain_id: DomainId,
        required_capability: BridgeCapabilityId,
        mut handler: Handler,
    ) -> Result<(), BridgeHostError>
    where
        Snapshot: Serialize + 'static,
        Handler: BridgeSnapshotHandler<Snapshot> + 'static,
    {
        self.validate_capability(&domain_id, &required_capability)?;
        if self.snapshots.contains_key(&domain_id) {
            return Err(registration_error(format!(
                "duplicate bridge snapshot registration: {domain_id}"
            )));
        }
        self.snapshots.insert(
            domain_id,
            RegisteredSnapshot {
                required_capability,
                handler: Mutex::new(Box::new(move |session_id, domain_id| {
                    transcode(handler.snapshot(session_id, domain_id)?)
                })),
            },
        );
        Ok(())
    }

    pub(crate) fn domains(&self) -> impl Iterator<Item = &DomainCapabilityDescriptor> {
        self.domains.values()
    }

    pub(crate) fn domain(&self, domain_id: &DomainId) -> Option<&DomainCapabilityDescriptor> {
        self.domains.get(domain_id)
    }

    pub(crate) fn route(&self, route: &str) -> Option<&RegisteredRoute> {
        self.routes.get(route)
    }

    pub(crate) fn snapshot(&self, domain_id: &DomainId) -> Option<&RegisteredSnapshot> {
        self.snapshots.get(domain_id)
    }

    fn insert_route(
        &mut self,
        route: String,
        registration: RegisteredRoute,
    ) -> Result<(), BridgeHostError> {
        validate_route(&route)?;
        self.validate_capability(&registration.domain_id, &registration.required_capability)?;
        if self.routes.contains_key(&route) {
            return Err(registration_error(format!(
                "duplicate bridge route registration: {route}"
            )));
        }
        self.routes.insert(route, registration);
        Ok(())
    }

    fn validate_capability(
        &self,
        domain_id: &DomainId,
        capability: &BridgeCapabilityId,
    ) -> Result<(), BridgeHostError> {
        let Some(domain) = self.domains.get(domain_id) else {
            return Err(registration_error(format!(
                "route references unregistered domain: {domain_id}"
            )));
        };
        if !domain.capabilities().contains(capability) {
            return Err(registration_error(format!(
                "route capability {capability} is not declared by domain {domain_id}"
            )));
        }
        Ok(())
    }
}

fn validate_route(route: &str) -> Result<(), BridgeHostError> {
    let valid = !route.is_empty()
        && route.len() <= MAXIMUM_ROUTE_BYTES
        && route.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b':' | b'-')
        });
    if valid {
        Ok(())
    } else {
        Err(registration_error(format!(
            "invalid bridge route registration: {route}"
        )))
    }
}

fn transcode<Source, Target>(source: Source) -> Result<Target, BridgeHostError>
where
    Source: Serialize,
    Target: DeserializeOwned,
{
    serde_json::from_value(
        serde_json::to_value(source).map_err(|error| BridgeHostError::codec(error.to_string()))?,
    )
    .map_err(|error| BridgeHostError::codec(error.to_string()))
}

fn registration_error(message: impl Into<String>) -> BridgeHostError {
    BridgeHostError::new(BridgeHostErrorCode::InvalidRegistration, message, false)
}
