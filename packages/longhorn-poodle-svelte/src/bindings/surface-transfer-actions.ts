import type {
  SurfaceSessionResponse,
  SurfaceSessionStartRequest,
  SurfaceTransferCommand,
  SurfaceTransferResponse,
} from "@inflatable-cookie/longhorn/surface-transfer";
import {
  LONGHORN_TRANSFER_MIME_TYPE,
  TRANSFER_PROTOCOL_VERSION,
  parseTransferPayload,
  serializeTransferPayload,
} from "@inflatable-cookie/longhorn/transfer";
import { notify, reportError, resolveDropSelector } from "./drop-target.ts";
import type { DropSelector } from "./drop-target.ts";

import type { SurfaceTransferState } from "./surface-transfer.svelte.ts";

export type SurfaceDropSelector = DropSelector;

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
  let removePointerListeners = noop;
  const priorDraggable = node.getAttribute("draggable");
  node.draggable = true;

  function cancel(): void {
    ++preparationGeneration;
    armedRequest = undefined;
    removePointerListeners();
    removePointerListeners = noop;
    void options.state.cancelPreparation().catch((error: unknown) => {
      reportError(options.reportError, error);
    });
  }

  function handlePointerDown(event: PointerEvent): void {
    if (event.button !== 0 || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) {
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
    void options.state
      .prepare(request)
      .then((response) => {
        notify(options.onPreparation, response, options.reportError);
        if (generation === preparationGeneration && response.status !== "started") {
          armedRequest = undefined;
        }
      })
      .catch((error: unknown) => {
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
      reportError(options.reportError, new StaleSurfaceTransferPreparationError());
      return;
    }
    removePointerListeners();
    removePointerListeners = noop;
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
      if (nextOptions.state !== options.state) {
        cancel();
      }
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

function handleDragOver(event: DragEvent): void {
  // The drag store exposes types during dragover and payload data on drop.
  const dataTransfer = event.dataTransfer;
  if (!dataTransfer?.types.includes(LONGHORN_TRANSFER_MIME_TYPE)) {
    return;
  }
  event.preventDefault();
  dataTransfer.dropEffect = "move";
}

export function surfaceTransferDrop(
  node: HTMLElement,
  initialOptions: SurfaceTransferDropOptions,
): SurfaceTransferAction<SurfaceTransferDropOptions> {
  let options = initialOptions;

  function handleDrop(event: DragEvent): void {
    const payload = readPayload(event.dataTransfer);
    if (payload === null) {
      return;
    }
    event.preventDefault();
    let request: SurfaceTransferCommand;
    try {
      request = {
        protocol_version: TRANSFER_PROTOCOL_VERSION,
        request_id: options.nextRequestId(),
        session_id: payload.session_id,
        selector: resolveDropSelector(options.selector, () => ({
          x: event.screenX,
          y: event.screenY,
        })),
      };
    } catch (error) {
      reportError(options.reportError, error);
      notify(options.onTerminal, undefined, options.reportError);
      return;
    }
    void options.state
      .commit(request)
      .then((response) => {
        notify(options.onResponse, response, options.reportError);
      })
      .catch((error: unknown) => {
        reportError(options.reportError, error);
      })
      .finally(() => {
        notify(options.onTerminal, undefined, options.reportError);
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
  if (dataTransfer === null || dataTransfer === undefined) {
    return null;
  }
  const raw = dataTransfer.getData(LONGHORN_TRANSFER_MIME_TYPE);
  if (raw.length === 0) {
    return null;
  }
  try {
    return parseTransferPayload(raw);
  } catch {
    return null;
  }
}

function noop(): void {}
