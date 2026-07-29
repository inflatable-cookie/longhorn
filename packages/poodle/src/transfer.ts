import {
  LONGHORN_TRANSFER_MIME_TYPE,
  TRANSFER_PROTOCOL_VERSION,
  parseTransferPayload,
  serializeTransferPayload,
  type PanelSessionStartRequest,
  type PanelTransferCommand,
  type PanelTransferResponse,
  type TransferCommitSelector,
  type TransferSessionResponse,
} from "@longhorn/transfer";
import type { TransferState } from "@longhorn/svelte/transfer";
import type {
  DockExternalDragSource,
  DockExternalDropTarget,
} from "@poodle/svelte";

export type PanelDropSelector =
  | {
      readonly kind: "explicit_zone";
      readonly dropZoneId: string;
    }
  | { readonly kind: "screen_point" };

export interface PanelTransferDragSourceOptions {
  readonly state: TransferState;
  readonly makeStartRequest: (
    panelInstanceId: string,
  ) => PanelSessionStartRequest;
  readonly reportError: (error: unknown) => void;
  readonly onPreparation?: (response: TransferSessionResponse) => void;
}

export interface PanelTransferDropTargetOptions {
  readonly state: TransferState;
  readonly selector: PanelDropSelector;
  readonly nextRequestId: () => string;
  readonly reportError: (error: unknown) => void;
  readonly onResponse?: (response: PanelTransferResponse) => void;
  readonly onTerminal?: () => void;
}

export function createPanelTransferDragSource(
  options: PanelTransferDragSourceOptions,
): DockExternalDragSource {
  return {
    async prepare(context) {
      if (
        context.event.altKey ||
        context.event.ctrlKey ||
        context.event.metaKey ||
        context.event.shiftKey
      ) {
        return null;
      }
      const request = options.makeStartRequest(context.panel.value);
      const response = await options.state.preparePanel(request);
      notify(
        options.onPreparation,
        response,
        options.reportError,
      );
      if (response.status !== "started") return null;

      const payload = response.session.payload;
      let terminal = false;
      const cancel = () => {
        if (terminal) return;
        terminal = true;
        void options.state.cancelPreparation().catch((error) => {
          report(options.reportError, error);
        });
      };

      return {
        start(startContext) {
          const current = options.state.preparation;
          if (
            current.status !== "prepared" ||
            current.response.session.payload.session_id !==
              payload.session_id ||
            current.response.session.request_id !== request.request_id ||
            startContext.panel.value !== request.panel_instance_id
          ) {
            startContext.event.preventDefault();
            cancel();
            report(
              options.reportError,
              new StalePanelTransferPreparationError(),
            );
            return;
          }
          startContext.dataTransfer.setData(
            LONGHORN_TRANSFER_MIME_TYPE,
            serializeTransferPayload(payload),
          );
          startContext.dataTransfer.effectAllowed = "move";
        },
        end: cancel,
        cancel,
      };
    },
    onPrepareError(error) {
      report(options.reportError, error);
    },
  };
}

export function createPanelTransferDropTarget(
  options: PanelTransferDropTargetOptions,
): DockExternalDropTarget {
  return {
    canDrop({ dataTransfer }) {
      return readPayload(dataTransfer) !== null;
    },
    async drop({ event, dataTransfer }) {
      try {
        const payload = readPayload(dataTransfer);
        if (payload === null) {
          throw new MissingPanelTransferPayloadError();
        }
        const request: PanelTransferCommand = {
          protocol_version: TRANSFER_PROTOCOL_VERSION,
          request_id: options.nextRequestId(),
          session_id: payload.session_id,
          selector: resolveSelector(options.selector, event),
        };
        const response = await options.state.commitPanel(request);
        notify(options.onResponse, response, options.reportError);
      } catch (error) {
        report(options.reportError, error);
      } finally {
        notifyTerminal(options.onTerminal, options.reportError);
      }
    },
  };
}

export class StalePanelTransferPreparationError extends Error {
  constructor() {
    super("panel dragstart has no matching prepared transfer session");
    this.name = "StalePanelTransferPreparationError";
  }
}

export class MissingPanelTransferPayloadError extends Error {
  constructor() {
    super("drop has no valid Longhorn transfer payload");
    this.name = "MissingPanelTransferPayloadError";
  }
}

function readPayload(dataTransfer: DataTransfer) {
  const raw = dataTransfer.getData(LONGHORN_TRANSFER_MIME_TYPE);
  if (raw.length === 0) return null;
  try {
    return parseTransferPayload(raw);
  } catch {
    return null;
  }
}

function resolveSelector(
  selector: PanelDropSelector,
  event: DragEvent,
): TransferCommitSelector {
  if (selector.kind === "explicit_zone") {
    return {
      kind: "explicit_zone",
      drop_zone_id: selector.dropZoneId,
    };
  }
  return {
    kind: "screen_point",
    point: screenPoint(event),
  };
}

function screenPoint(event: DragEvent): { x: number; y: number } {
  if (
    !Number.isSafeInteger(event.screenX) ||
    !Number.isSafeInteger(event.screenY)
  ) {
    throw new RangeError("drop screen point must use integer screen DIPs");
  }
  return { x: event.screenX, y: event.screenY };
}

function report(reporter: (error: unknown) => void, error: unknown): void {
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
    report(reporter, error);
  }
}

function notifyTerminal(
  listener: (() => void) | undefined,
  reporter: (error: unknown) => void,
): void {
  try {
    listener?.();
  } catch (error) {
    report(reporter, error);
  }
}
