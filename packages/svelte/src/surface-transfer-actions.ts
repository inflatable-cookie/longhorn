import type {
  SurfaceSessionResponse,
  SurfaceSessionStartRequest,
  SurfaceTransferCommand,
  SurfaceTransferResponse,
} from "@inflatable-cookie/longhorn-surface-transfer";
import {
  LONGHORN_TRANSFER_MIME_TYPE,
  TRANSFER_PROTOCOL_VERSION,
  parseTransferPayload,
  serializeTransferPayload,
  type TransferCommitSelector,
} from "@inflatable-cookie/longhorn-transfer";

import type { SurfaceTransferState } from "./surface-transfer.svelte.ts";

export type SurfaceDropSelector =
  | {
      readonly kind: "explicit_zone";
      readonly dropZoneId: string;
    }
  | { readonly kind: "screen_point" };

export interface SurfaceTransferDragOptions {
  readonly state: SurfaceTransferState;
  readonly makeStartRequest: () => SurfaceSessionStartRequest;
  readonly reportError: (error: unknown) => void;
  readonly onPreparation?: (response: SurfaceSessionResponse) => void;
}

export interface SurfaceTransferDropOptions {
  readonly state: SurfaceTransferState;
  readonly selector: SurfaceDropSelector;
  readonly nextRequestId: () => string;
  readonly reportError: (error: unknown) => void;
  readonly onResponse?: (response: SurfaceTransferResponse) => void;
  readonly onTerminal?: () => void;
}

export interface SurfaceTransferAction<T> {
  update(options: T): void;
  destroy(): void;
}

export function surfaceTransferDrag(
  node: HTMLElement,
  initialOptions: SurfaceTransferDragOptions,
): SurfaceTransferAction<SurfaceTransferDragOptions> {
  let options = initialOptions;
  let preparationGeneration = 0;
  let armedRequest: SurfaceSessionStartRequest | undefined;
  let removePointerListeners = () => undefined;
  const priorDraggable = node.getAttribute("draggable");
  node.draggable = true;

  function cancel(): void {
    ++preparationGeneration;
    armedRequest = undefined;
    removePointerListeners();
    removePointerListeners = () => undefined;
    void options.state.cancelPreparation().catch((error) => {
      reportError(options.reportError, error);
    });
  }

  function handlePointerDown(event: PointerEvent): void {
    if (
      event.button !== 0 ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      event.shiftKey
    ) {
      return;
    }
    cancel();
    const generation = ++preparationGeneration;
    const request = options.makeStartRequest();
    armedRequest = request;
    const ownerWindow = node.ownerDocument.defaultView;
    if (ownerWindow) {
      const released = () => cancel();
      ownerWindow.addEventListener("pointerup", released, true);
      ownerWindow.addEventListener("pointercancel", released, true);
      removePointerListeners = () => {
        ownerWindow.removeEventListener("pointerup", released, true);
        ownerWindow.removeEventListener("pointercancel", released, true);
      };
    }
    void options.state.prepare(request).then((response) => {
      notify(options.onPreparation, response, options.reportError);
      if (
        generation === preparationGeneration &&
        response.status !== "started"
      ) {
        armedRequest = undefined;
      }
    }).catch((error) => {
      if (generation === preparationGeneration) {
        reportError(options.reportError, error);
      }
    });
  }

  function handleDragStart(event: DragEvent): void {
    const request = armedRequest;
    const preparation = options.state.preparation;
    if (
      !event.dataTransfer ||
      request === undefined ||
      preparation.status !== "prepared" ||
      preparation.response.status !== "started" ||
      preparation.response.session.request_id !== request.request_id
    ) {
      event.preventDefault();
      cancel();
      reportError(
        options.reportError,
        new StaleSurfaceTransferPreparationError(),
      );
      return;
    }
    removePointerListeners();
    removePointerListeners = () => undefined;
    event.dataTransfer.setData(
      LONGHORN_TRANSFER_MIME_TYPE,
      serializeTransferPayload(preparation.response.session.payload),
    );
    event.dataTransfer.effectAllowed = "move";
  }

  node.addEventListener("pointerdown", handlePointerDown);
  node.addEventListener("dragstart", handleDragStart);
  node.addEventListener("dragend", cancel);

  return {
    update(nextOptions) {
      cancel();
      options = nextOptions;
    },
    destroy() {
      node.removeEventListener("pointerdown", handlePointerDown);
      node.removeEventListener("dragstart", handleDragStart);
      node.removeEventListener("dragend", cancel);
      cancel();
      if (priorDraggable === null) {
        node.removeAttribute("draggable");
      } else {
        node.setAttribute("draggable", priorDraggable);
      }
    },
  };
}

export function surfaceTransferDrop(
  node: HTMLElement,
  initialOptions: SurfaceTransferDropOptions,
): SurfaceTransferAction<SurfaceTransferDropOptions> {
  let options = initialOptions;

  function handleDragOver(event: DragEvent): void {
    if (readPayload(event.dataTransfer) === null) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
  }

  function handleDrop(event: DragEvent): void {
    const payload = readPayload(event.dataTransfer);
    if (payload === null) return;
    event.preventDefault();
    let request: SurfaceTransferCommand;
    try {
      request = {
        protocol_version: TRANSFER_PROTOCOL_VERSION,
        request_id: options.nextRequestId(),
        session_id: payload.session_id,
        selector: resolveSelector(options.selector, event),
      };
    } catch (error) {
      reportError(options.reportError, error);
      notifyTerminal(options.onTerminal, options.reportError);
      return;
    }
    void options.state.commit(request)
      .then((response) => {
        notify(options.onResponse, response, options.reportError);
      })
      .catch((error) => {
        reportError(options.reportError, error);
      })
      .finally(() => {
        notifyTerminal(options.onTerminal, options.reportError);
      });
  }

  node.addEventListener("dragover", handleDragOver);
  node.addEventListener("drop", handleDrop);
  return {
    update(nextOptions) {
      options = nextOptions;
    },
    destroy() {
      node.removeEventListener("dragover", handleDragOver);
      node.removeEventListener("drop", handleDrop);
    },
  };
}

export class StaleSurfaceTransferPreparationError extends Error {
  constructor() {
    super("Surface dragstart has no matching prepared transfer session");
    this.name = "StaleSurfaceTransferPreparationError";
  }
}

function readPayload(dataTransfer: DataTransfer | null | undefined) {
  if (dataTransfer == null) return null;
  const raw = dataTransfer.getData(LONGHORN_TRANSFER_MIME_TYPE);
  if (raw.length === 0) return null;
  try {
    return parseTransferPayload(raw);
  } catch {
    return null;
  }
}

function resolveSelector(
  selector: SurfaceDropSelector,
  event: DragEvent,
): TransferCommitSelector {
  if (selector.kind === "explicit_zone") {
    return {
      kind: "explicit_zone",
      drop_zone_id: selector.dropZoneId,
    };
  }
  if (
    !Number.isSafeInteger(event.screenX) ||
    !Number.isSafeInteger(event.screenY)
  ) {
    throw new RangeError("drop screen point must use integer screen DIPs");
  }
  return {
    kind: "screen_point",
    point: { x: event.screenX, y: event.screenY },
  };
}

function reportError(
  reporter: (error: unknown) => void,
  error: unknown,
): void {
  try {
    reporter(error);
  } catch {
    // Reporting failure must not escape an event callback.
  }
}

function notify<T>(
  listener: ((value: T) => void) | undefined,
  value: T,
  reporter: (error: unknown) => void,
): void {
  try {
    listener?.(value);
  } catch (error) {
    reportError(reporter, error);
  }
}

function notifyTerminal(
  listener: (() => void) | undefined,
  reporter: (error: unknown) => void,
): void {
  try {
    listener?.();
  } catch (error) {
    reportError(reporter, error);
  }
}
