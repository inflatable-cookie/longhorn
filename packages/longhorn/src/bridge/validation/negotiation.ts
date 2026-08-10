import { BRIDGE_FIELDS } from "../generated/fields.ts";
import {
  BRIDGE_MAXIMUM_AUTHORITY_DOMAINS,
  BRIDGE_MAXIMUM_CAPABILITIES_PER_DOMAIN,
  BRIDGE_MAXIMUM_CAPABILITY_DOMAINS,
  BRIDGE_MAXIMUM_DIAGNOSTIC_MESSAGE_BYTES,
  BRIDGE_MAXIMUM_DIAGNOSTICS,
  BRIDGE_MAXIMUM_REQUESTED_DOMAINS,
  BRIDGE_MAXIMUM_TRANSPORT_FEATURES,
  BRIDGE_AUTHENTICATION_POSTURES,
  BRIDGE_ADMITTED_CONNECTION_REASONS,
  BRIDGE_CONNECTION_REASONS,
  BRIDGE_CONNECTION_STATES,
  BRIDGE_DOMAIN_AVAILABILITIES,
  BRIDGE_EXECUTION_AUTHORITIES,
  BRIDGE_HOST_FORMS,
  BRIDGE_PROTOCOL_VERSION,
  BRIDGE_READ_AUTHORITIES,
  BRIDGE_WRITE_AUTHORITIES,
  type BridgeConnectionReason,
  type BridgeConnectionState,
  type BridgeConnectionStatus,
  type BridgeDiagnostic,
  type BridgeHelloRequest,
  type BridgeHostDescriptor,
  type BridgeNegotiationReceipt,
  type DomainAuthorityDescriptor,
  type DomainCapabilityDescriptor,
} from "../generated/protocol.ts";
import {
  array,
  domainId,
  incompatible,
  integer,
  nullable,
  oneOf,
  opaqueId,
  record,
  unique,
} from "./base.ts";

export function assertBridgeProtocolVersion(value: unknown): asserts value is 1 {
  if (value !== BRIDGE_PROTOCOL_VERSION) {
    incompatible("unsupported_protocol_version", value);
  }
}

export function parseBridgeHelloRequest(value: unknown): BridgeHelloRequest {
  const source = record(value, BRIDGE_FIELDS.BridgeHelloRequest);
  assertBridgeProtocolVersion(source.protocolVersion);
  const requestedDomains = array(source.requestedDomains, BRIDGE_MAXIMUM_REQUESTED_DOMAINS).map(domainId);
  unique(requestedDomains, String);
  return {
    protocolVersion: BRIDGE_PROTOCOL_VERSION,
    bridgeId: opaqueId(source.bridgeId),
    requestedDomains,
  };
}

export function parseBridgeNegotiationReceipt(
  value: unknown,
  request?: BridgeHelloRequest,
): BridgeNegotiationReceipt {
  const source = record(value, BRIDGE_FIELDS.BridgeNegotiationReceipt);
  assertBridgeProtocolVersion(source.protocolVersion);
  const capabilities = array(source.domainCapabilities, BRIDGE_MAXIMUM_CAPABILITY_DOMAINS).map(
    parseDomainCapability,
  );
  const authorities = array(source.domainAuthorities, BRIDGE_MAXIMUM_AUTHORITY_DOMAINS).map(
    parseDomainAuthority,
  );
  const connection = parseConnectionStatus(source.connection);
  if (connection.state !== "ready") {
    incompatible("invalid_connection_status", connection);
  }
  const features = array(source.transportFeatures, BRIDGE_MAXIMUM_TRANSPORT_FEATURES).map(opaqueId);
  const diagnostics = array(source.diagnostics, BRIDGE_MAXIMUM_DIAGNOSTICS).map(parseDiagnostic);
  unique(features, String);
  unique(capabilities, (item) => item.domainId);
  unique(authorities, (item) => item.domainId);
  unique(diagnostics, (item) => item.diagnosticId);

  const capabilityDomains = new Set(
    capabilities.map((item) => item.domainId),
  );
  const writerScopes = new Set<string>();
  for (const authority of authorities) {
    if (!capabilityDomains.has(authority.domainId)) {
      incompatible("authority_without_capability", authority);
    }
    if (authority.writeAuthority === "authoritative") {
      if (writerScopes.has(authority.scopeId)) {
        incompatible("multiple_writers", authority.scopeId);
      }
      writerScopes.add(authority.scopeId);
    }
  }
  if (request !== undefined) {
    const requested = new Set(request.requestedDomains);
    for (const capability of capabilities) {
      if (!requested.has(capability.domainId)) {
        incompatible("unrequested_domain", capability.domainId);
      }
    }
  }

  return {
    protocolVersion: BRIDGE_PROTOCOL_VERSION,
    host: parseHost(source.host),
    sessionId: opaqueId(source.sessionId),
    connection,
    authentication: oneOf(
      source.authentication,
      BRIDGE_AUTHENTICATION_POSTURES,
      "unknown_authentication_posture",
    ),
    transportFeatures: features,
    domainCapabilities: capabilities,
    domainAuthorities: authorities,
    diagnostics,
  };
}

function parseHost(value: unknown): BridgeHostDescriptor {
  const source = record(value, BRIDGE_FIELDS.BridgeHostDescriptor);
  return {
    hostInstanceId: opaqueId(source.hostInstanceId),
    form: oneOf(source.form, BRIDGE_HOST_FORMS, "unknown_host_form"),
  };
}

function parseConnectionStatus(value: unknown): BridgeConnectionStatus {
  const source = record(value, BRIDGE_FIELDS.BridgeConnectionStatus);
  const state = oneOf(
    source.state,
    BRIDGE_CONNECTION_STATES,
    "unknown_connection_state",
  );
  const reason = nullable(source.reason, (candidate) =>
    oneOf(
      candidate,
      BRIDGE_CONNECTION_REASONS,
      "invalid_connection_status",
    )
  );
  if (!validConnectionReason(state, reason)) {
    incompatible("invalid_connection_status", value);
  }
  return { state, reason };
}

function validConnectionReason(
  state: BridgeConnectionState,
  reason: BridgeConnectionReason | null,
): boolean {
  // Generated from `BridgeConnectionStatus::ADMITTED_REASONS`. This used to be
  // an eleven-arm literal here, and Card 160 recorded it as the one rule that
  // could not be derived because the pairing exists in no type. That was
  // wrong: it exists in Rust, in a `matches!` arm, which `ts-rs` cannot carry
  // because it is a relation between two enums rather than a type. The two
  // copies agreed arm for arm — by maintenance, not by construction.
  return BRIDGE_ADMITTED_CONNECTION_REASONS[state].includes(reason);
}

function parseDomainCapability(value: unknown): DomainCapabilityDescriptor {
  const source = record(value, BRIDGE_FIELDS.DomainCapabilityDescriptor);
  const capabilities = array(source.capabilities, BRIDGE_MAXIMUM_CAPABILITIES_PER_DOMAIN).map(opaqueId);
  if (capabilities.length === 0) {
    incompatible("invalid_array", value);
  }
  unique(capabilities, String);
  return {
    domainId: domainId(source.domainId),
    capabilities,
  };
}

function parseDomainAuthority(value: unknown): DomainAuthorityDescriptor {
  const source = record(value, BRIDGE_FIELDS.DomainAuthorityDescriptor);
  const availability = oneOf(
    source.availability,
    BRIDGE_DOMAIN_AVAILABILITIES,
    "unknown_availability",
  );
  const readAuthority = oneOf(
    source.readAuthority,
    BRIDGE_READ_AUTHORITIES,
    "unknown_read_authority",
  );
  const writeAuthority = oneOf(
    source.writeAuthority,
    BRIDGE_WRITE_AUTHORITIES,
    "unknown_write_authority",
  );
  const executionAuthority = oneOf(
    source.executionAuthority,
    BRIDGE_EXECUTION_AUTHORITIES,
    "unknown_execution_authority",
  );
  const authoritativeRevision = nullable(
    source.authoritativeRevision,
    integer,
  );
  const ownsAnything = readAuthority !== "none" ||
    writeAuthority !== "none" ||
    executionAuthority !== "none";
  const revisionIsAuthoritative = readAuthority === "authoritative" ||
    writeAuthority === "authoritative";
  if (
    (availability === "offline" &&
      (ownsAnything || authoritativeRevision !== null)) ||
    (authoritativeRevision !== null && !revisionIsAuthoritative)
  ) {
    incompatible("invalid_authority_descriptor", value);
  }
  return {
    domainId: domainId(source.domainId),
    scopeId: opaqueId(source.scopeId),
    availability,
    readAuthority,
    writeAuthority,
    executionAuthority,
    authorityEpoch: integer(source.authorityEpoch, 1),
    authoritativeRevision,
  };
}

function parseDiagnostic(value: unknown): BridgeDiagnostic {
  const source = record(value, BRIDGE_FIELDS.BridgeDiagnostic);
  if (
    typeof source.message !== "string" ||
    source.message.length === 0 ||
    new TextEncoder().encode(source.message).length >
      BRIDGE_MAXIMUM_DIAGNOSTIC_MESSAGE_BYTES
  ) {
    incompatible("invalid_message", source.message);
  }
  return {
    diagnosticId: opaqueId(source.diagnosticId),
    message: source.message,
  };
}
