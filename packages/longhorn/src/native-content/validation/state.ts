import { NATIVE_CONTENT_FIELDS } from "../generated/fields.ts";
import {
  NATIVE_CONTENT_ATTACHMENT_LIFECYCLES,
  NATIVE_CONTENT_CHANGE_KINDS,
  NATIVE_CONTENT_DESIRED_PRESENCE,
  NATIVE_CONTENT_DESIRED_VISIBILITY_STATES,
  NATIVE_CONTENT_DETACH_POLICIES,
  NATIVE_CONTENT_EFFECTIVE_FOCUS_STATES,
  NATIVE_CONTENT_EFFECTIVE_VISIBILITY_STATES,
  NATIVE_CONTENT_FOCUS_INTENTS,
  NATIVE_CONTENT_HOST_DESTROY_OUTCOMES,
  NATIVE_CONTENT_INPUT_ROUTING_MODES,
  NATIVE_CONTENT_MECHANISMS,
  NATIVE_CONTENT_OBSERVED_GEOMETRY_KINDS,
  NATIVE_CONTENT_OPERATION_KINDS,
  NATIVE_CONTENT_OPERATION_OUTCOME_KINDS,
  NATIVE_CONTENT_READINESS_STATES,
  NATIVE_CONTENT_SIZE_DECISION_KINDS,
  type ContentSizeDecision,
  type ContentSizeProposal,
  type DesiredUpdate,
  type NativeContentChangedEvent,
  type NativeContentCursor,
  type NativeContentSnapshot,
} from "../generated/protocol.ts";
import {
  array,
  assertNativeContentProtocolVersion,
  assertProductPayloadFree,
  boolean,
  exactKeys,
  variantKeys,
  exactObject,
  fail,
  finite,
  member,
  natural,
  nullable,
  opaqueId,
  positive,
  record,
} from "./base.ts";

export function assertValidNativeContentSnapshot(
  value: unknown,
): asserts value is NativeContentSnapshot {
  assertProductPayloadFree(value);
  const object = exactObject(value, "$", NATIVE_CONTENT_FIELDS.NativeContentSnapshot);
  assertNativeContentProtocolVersion(object.protocol_version);
  cursor(object.cursor, "$.cursor");
  desiredState(object.desired, "$.desired");
  observedState(object.observed, "$.observed");
  nullable(object.invalidated_generation, "$.invalidated_generation", positive);
  const desired = object.desired as Record<string, unknown>;
  const observed = object.observed as Record<string, unknown>;
  const valueCursor = object.cursor as Record<string, unknown>;
  if (
    desired.island_id !== valueCursor.island_id ||
    desired.generation !== valueCursor.attach_generation ||
    desired.revision !== valueCursor.desired_revision ||
    observed.revision !== valueCursor.observed_revision
  ) {
    fail("$.cursor", "snapshot cursor does not match state");
  }
}

export function assertValidNativeContentChangedEvent(
  value: unknown,
): asserts value is NativeContentChangedEvent {
  assertProductPayloadFree(value);
  const object = exactObject(value, "$", NATIVE_CONTENT_FIELDS.NativeContentChangedEvent);
  assertNativeContentProtocolVersion(object.protocol_version);
  cursor(object.cursor, "$.cursor");
  change(object.change, "$.change");
}

export function assertValidDesiredUpdate(
  value: unknown,
): asserts value is DesiredUpdate {
  desiredUpdate(value, "$", false);
}

export function assertValidContentSizeProposal(
  value: unknown,
): asserts value is ContentSizeProposal {
  proposal(value, "$");
}

export function assertValidContentSizeDecision(
  value: unknown,
): asserts value is ContentSizeDecision {
  decision(value, "$");
}

export function cursor(value: unknown, path: string): asserts value is NativeContentCursor {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.NativeContentCursor);
  positive(object.authority_epoch, `${path}.authority_epoch`);
  positive(object.client_epoch, `${path}.client_epoch`);
  opaqueId(object.island_id, `${path}.island_id`);
  positive(object.attach_generation, `${path}.attach_generation`);
  natural(object.desired_revision, `${path}.desired_revision`);
  natural(object.observed_revision, `${path}.observed_revision`);
}

export function proposal(
  value: unknown,
  path: string,
): asserts value is ContentSizeProposal {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.ContentSizeProposal);
  positive(object.generation, `${path}.generation`);
  natural(object.desired_revision, `${path}.desired_revision`);
  clientSize(object.size, `${path}.size`);
}

export function decision(
  value: unknown,
  path: string,
): asserts value is ContentSizeDecision {
  const object = record(value, path);
  member(object.kind, NATIVE_CONTENT_SIZE_DECISION_KINDS, `${path}.kind`);
  if (object.kind === "accepted") {
    exactKeys(object, path, variantKeys("ContentSizeDecision", object, path));
  } else if (object.kind === "constrained") {
    exactKeys(object, path, variantKeys("ContentSizeDecision", object, path));
    clientSize(object.size, `${path}.size`);
  } else {
    exactKeys(object, path, variantKeys("ContentSizeDecision", object, path));
    opaqueId(object.code, `${path}.code`);
  }
}

export function desiredUpdate(
  value: unknown,
  path: string,
  includeIdentity: boolean,
): void {
  const keys = [
    "generation",
    "host_window_id",
    "viewport",
    "scale",
    "rounding",
    "presence",
    "visibility",
    "focus",
    "input_routing",
  ];
  if (includeIdentity) keys.unshift("island_id", "kind_id", "capabilities", "revision");
  const object = exactObject(value, path, keys);
  if (includeIdentity) {
    opaqueId(object.island_id, `${path}.island_id`);
    opaqueId(object.kind_id, `${path}.kind_id`);
    capabilities(object.capabilities, `${path}.capabilities`);
    natural(object.revision, `${path}.revision`);
  }
  positive(object.generation, `${path}.generation`);
  opaqueId(object.host_window_id, `${path}.host_window_id`);
  clientRect(object.viewport, `${path}.viewport`);
  positive(object.scale, `${path}.scale`);
  member(object.rounding, ["floor", "ceil", "nearest"] as const, `${path}.rounding`);
  member(object.presence, NATIVE_CONTENT_DESIRED_PRESENCE, `${path}.presence`);
  desiredVisibility(object.visibility, `${path}.visibility`);
  member(object.focus, NATIVE_CONTENT_FOCUS_INTENTS, `${path}.focus`);
  member(object.input_routing, NATIVE_CONTENT_INPUT_ROUTING_MODES, `${path}.input_routing`);
}

function desiredState(value: unknown, path: string): void {
  desiredUpdate(value, path, true);
}

function capabilities(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.MechanismCapabilities);
  member(object.mechanism, NATIVE_CONTENT_MECHANISMS, `${path}.mechanism`);
  member(object.active_input_routing, NATIVE_CONTENT_INPUT_ROUTING_MODES, `${path}.active_input_routing`);
  boolean(object.accepts_content_size_requests, `${path}.accepts_content_size_requests`);
  member(object.detach_policy, NATIVE_CONTENT_DETACH_POLICIES, `${path}.detach_policy`);
  boolean(object.observes_visibility, `${path}.observes_visibility`);
  boolean(object.observes_focus, `${path}.observes_focus`);
}

function desiredVisibility(value: unknown, path: string): void {
  const object = record(value, path);
  member(object.state, NATIVE_CONTENT_DESIRED_VISIBILITY_STATES, `${path}.state`);
  if (object.state === "visible") {
    exactKeys(object, path, variantKeys("DesiredVisibility", object, path));
  } else {
    exactKeys(object, path, variantKeys("DesiredVisibility", object, path));
    opaqueId(object.reason, `${path}.reason`);
  }
}

function observedState(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.ObservedState);
  natural(object.revision, `${path}.revision`);
  positive(object.generation, `${path}.generation`);
  member(object.lifecycle, NATIVE_CONTENT_ATTACHMENT_LIFECYCLES, `${path}.lifecycle`);
  member(object.readiness, NATIVE_CONTENT_READINESS_STATES, `${path}.readiness`);
  member(object.visibility, NATIVE_CONTENT_EFFECTIVE_VISIBILITY_STATES, `${path}.visibility`);
  member(object.focus, NATIVE_CONTENT_EFFECTIVE_FOCUS_STATES, `${path}.focus`);
  observedGeometry(object.geometry, `${path}.geometry`);
  nullable(object.input_routing, `${path}.input_routing`, (candidate, candidatePath) =>
    member(candidate, NATIVE_CONTENT_INPUT_ROUTING_MODES, candidatePath)
  );
}

function observedGeometry(value: unknown, path: string): void {
  const object = record(value, path);
  member(object.kind, NATIVE_CONTENT_OBSERVED_GEOMETRY_KINDS, `${path}.kind`);
  switch (object.kind) {
    case "unknown":
      exactKeys(object, path, variantKeys("ObservedGeometry", object, path));
      break;
    case "child_bounds":
      exactKeys(object, path, variantKeys("ObservedGeometry", object, path));
      physicalRect(object.bounds, `${path}.bounds`);
      break;
    case "isolated_content":
      exactKeys(object, path, variantKeys("ObservedGeometry", object, path));
      physicalSize(object.size, `${path}.size`);
      break;
    case "backing_surface":
      exactKeys(object, path, variantKeys("ObservedGeometry", object, path));
      physicalRect(object.storage_bounds, `${path}.storage_bounds`);
      physicalRect(object.clip, `${path}.clip`);
      break;
  }
}

function change(value: unknown, path: string): void {
  const object = record(value, path);
  member(object.kind, NATIVE_CONTENT_CHANGE_KINDS, `${path}.kind`);
  switch (object.kind) {
    case "desired_updated":
      exactKeys(object, path, variantKeys("NativeContentChangeProjection", object, path));
      opaqueId(object.request_id, `${path}.request_id`);
      desiredReceipt(object.receipt, `${path}.receipt`);
      break;
    case "observation_admitted":
      exactKeys(object, path, variantKeys("NativeContentChangeProjection", object, path));
      nullable(object.request_id, `${path}.request_id`, opaqueId);
      observationReceipt(object.receipt, `${path}.receipt`);
      break;
    case "content_size_proposed":
      exactKeys(object, path, variantKeys("NativeContentChangeProjection", object, path));
      opaqueId(object.request_id, `${path}.request_id`);
      proposal(object.proposal, `${path}.proposal`);
      break;
    case "content_size_decided":
      exactKeys(object, path, variantKeys("NativeContentChangeProjection", object, path));
      opaqueId(object.request_id, `${path}.request_id`);
      proposalReceipt(object.receipt, `${path}.receipt`);
      break;
    case "apply_completed":
      exactKeys(object, path, variantKeys("NativeContentChangeProjection", object, path));
      opaqueId(object.request_id, `${path}.request_id`);
      applyReceipt(object.receipt, `${path}.receipt`);
      break;
    case "host_destroyed":
      exactKeys(object, path, variantKeys("NativeContentChangeProjection", object, path));
      nullable(object.request_id, `${path}.request_id`, opaqueId);
      hostDestroyReceipt(object.receipt, `${path}.receipt`);
      break;
  }
}

export function desiredReceipt(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.DesiredUpdateReceipt);
  natural(object.previous_revision, `${path}.previous_revision`);
  natural(object.current_revision, `${path}.current_revision`);
  positive(object.generation, `${path}.generation`);
}

export function proposalReceipt(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.ContentSizeProposalReceipt);
  proposal(object.proposal, `${path}.proposal`);
  decision(object.decision, `${path}.decision`);
  nullable(object.accepted_size, `${path}.accepted_size`, clientSize);
}

function observationReceipt(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.ObservationReceipt);
  natural(object.previous_revision, `${path}.previous_revision`);
  natural(object.current_revision, `${path}.current_revision`);
  positive(object.generation, `${path}.generation`);
  member(object.lifecycle, NATIVE_CONTENT_ATTACHMENT_LIFECYCLES, `${path}.lifecycle`);
}

function hostDestroyReceipt(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.HostDestroyReceipt);
  natural(object.previous_observed_revision, `${path}.previous_observed_revision`);
  natural(object.current_observed_revision, `${path}.current_observed_revision`);
  positive(object.generation, `${path}.generation`);
  member(object.outcome, NATIVE_CONTENT_HOST_DESTROY_OUTCOMES, `${path}.outcome`);
}

function applyReceipt(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.ApplyReceipt);
  opaqueId(object.island_id, `${path}.island_id`);
  natural(object.desired_revision, `${path}.desired_revision`);
  natural(object.observed_revision, `${path}.observed_revision`);
  positive(object.generation, `${path}.generation`);
  const steps = array(object.steps, `${path}.steps`);
  if (steps.length > 5) fail(`${path}.steps`, "apply receipt exceeds five steps");
  steps.forEach((step, index) => stepReceipt(step, `${path}.steps[${index}]`));
}

function stepReceipt(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.StepReceipt);
  positive(object.step, `${path}.step`);
  operation(object.operation, `${path}.operation`);
  outcome(object.outcome, `${path}.outcome`);
}

function operation(value: unknown, path: string): void {
  const object = record(value, path);
  member(object.kind, NATIVE_CONTENT_OPERATION_KINDS, `${path}.kind`);
  switch (object.kind) {
    case "attach":
      exactKeys(object, path, variantKeys("NativeContentOperation", object, path));
      opaqueId(object.host_window_id, `${path}.host_window_id`);
      member(object.mechanism, NATIVE_CONTENT_MECHANISMS, `${path}.mechanism`);
      break;
    case "set_child_bounds":
      exactKeys(object, path, variantKeys("NativeContentOperation", object, path));
      physicalRect(object.bounds, `${path}.bounds`);
      break;
    case "set_isolated_content_size":
      exactKeys(object, path, variantKeys("NativeContentOperation", object, path));
      physicalSize(object.size, `${path}.size`);
      break;
    case "set_backing_viewport":
      exactKeys(object, path, variantKeys("NativeContentOperation", object, path));
      physicalRect(object.clip, `${path}.clip`);
      break;
    case "hide":
      exactKeys(object, path, variantKeys("NativeContentOperation", object, path));
      opaqueId(object.reason, `${path}.reason`);
      break;
    case "set_input_routing":
      exactKeys(object, path, variantKeys("NativeContentOperation", object, path));
      member(object.mode, NATIVE_CONTENT_INPUT_ROUTING_MODES, `${path}.mode`);
      break;
    case "detach":
      exactKeys(object, path, variantKeys("NativeContentOperation", object, path));
      member(object.policy, NATIVE_CONTENT_DETACH_POLICIES, `${path}.policy`);
      break;
    case "show":
    case "request_focus":
    case "release_focus_if_owned":
      exactKeys(object, path, variantKeys("NativeContentOperation", object, path));
      break;
  }
}

function outcome(value: unknown, path: string): void {
  const object = record(value, path);
  member(object.kind, NATIVE_CONTENT_OPERATION_OUTCOME_KINDS, `${path}.kind`);
  if (object.kind === "failed") {
    exactKeys(object, path, variantKeys("OperationOutcome", object, path));
    opaqueId(object.code, `${path}.code`);
  } else if (object.kind === "dependency_skipped") {
    exactKeys(object, path, variantKeys("OperationOutcome", object, path));
    positive(object.blocked_by, `${path}.blocked_by`);
  } else {
    exactKeys(object, path, variantKeys("OperationOutcome", object, path));
  }
}

function clientRect(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.ClientRect);
  const origin = exactObject(object.origin, `${path}.origin`, NATIVE_CONTENT_FIELDS.ClientPoint);
  finite(origin.x, `${path}.origin.x`);
  finite(origin.y, `${path}.origin.y`);
  clientSize(object.size, `${path}.size`);
}

function clientSize(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.ClientSize);
  finite(object.width, `${path}.width`);
  finite(object.height, `${path}.height`);
  if ((object.width as number) < 0 || (object.height as number) < 0) {
    fail(path, "contains negative extent");
  }
}

function physicalRect(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.ClientRect);
  const origin = exactObject(object.origin, `${path}.origin`, NATIVE_CONTENT_FIELDS.ClientPoint);
  integer(origin.x, `${path}.origin.x`);
  integer(origin.y, `${path}.origin.y`);
  physicalSize(object.size, `${path}.size`);
}

function physicalSize(value: unknown, path: string): void {
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.ClientSize);
  natural(object.width, `${path}.width`);
  natural(object.height, `${path}.height`);
}

function integer(value: unknown, path: string): void {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) {
    fail(path, "expected safe integer");
  }
}
