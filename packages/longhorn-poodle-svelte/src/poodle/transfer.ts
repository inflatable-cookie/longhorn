import {
  CROSS_WINDOW_DRAG_PROTOCOL_VERSION,
  decodeDockPanelSubject,
  type CrossWindowDragReceipt,
  type CrossWindowDragSourceBridge,
  type CrossWindowDragTargetBridge,
  type CrossWindowDragTargetEvent,
  type DragDropCommitResult,
} from "@inflatable-cookie/poodle-core";
import {
  TRANSFER_PROTOCOL_VERSION,
  type PanelSessionStartRequest,
  type PanelTransferCommand,
  type PanelTransferResponse,
  type TransferCommitSelector,
  type TransferSessionResponse,
} from "@inflatable-cookie/longhorn/transfer";
import type { TransferState } from "@inflatable-cookie/longhorn-poodle-svelte/transfer";

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
  /** Required when `selector.kind` is `screen_point`. */
  readonly screenPoint?: () => { readonly x: number; readonly y: number };
}

export function createPanelTransferDragSource(
  options: PanelTransferDragSourceOptions,
): CrossWindowDragSourceBridge {
  return {
    capabilities: {
      pointer: true,
      touch: false,
      keyboardTargetPicker: false,
    },
    async prepare(request, signal) {
      if (signal.aborted) return null;
      const decoded = decodeDockPanelSubject(request.subject.id);
      if (decoded === null) return null;

      const startRequest = options.makeStartRequest(decoded.panelId);
      const response = await options.state.preparePanel(startRequest);
      notify(options.onPreparation, response, options.reportError);
      if (response.status !== "started") return null;

      return receiptFor(response.session.payload.session_id);
    },
    start() {
      return () => undefined;
    },
    cancel() {
      void options.state.cancelPreparation().catch((error) => {
        report(options.reportError, error);
      });
    },
  };
}

export function createPanelTransferDropTarget(
  options: PanelTransferDropTargetOptions,
): CrossWindowDragTargetBridge {
  const listeners = new Set<(event: CrossWindowDragTargetEvent) => void>();
  return {
    capabilities: {
      pointer: true,
      touch: false,
      keyboardTargetPicker: false,
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    async commit(request, signal) {
      if (signal.aborted) {
        return { status: "rejected", reason: "aborted" };
      }
      try {
        const transferRequest: PanelTransferCommand = {
          protocol_version: TRANSFER_PROTOCOL_VERSION,
          request_id: options.nextRequestId(),
          session_id: request.receipt.token,
          selector: resolveSelector(options),
        };
        const response = await options.state.commitPanel(transferRequest);
        notify(options.onResponse, response, options.reportError);
        return commitResult(response);
      } catch (error) {
        report(options.reportError, error);
        return {
          status: "failed",
          reason: error instanceof Error ? error.message : String(error),
        };
      } finally {
        notifyTerminal(options.onTerminal, options.reportError);
      }
    },
  };
}

function receiptFor(sessionId: string): CrossWindowDragReceipt {
  return {
    protocolVersion: CROSS_WINDOW_DRAG_PROTOCOL_VERSION,
    token: sessionId,
  };
}

function resolveSelector(
  options: PanelTransferDropTargetOptions,
): TransferCommitSelector {
  if (options.selector.kind === "explicit_zone") {
    return {
      kind: "explicit_zone",
      drop_zone_id: options.selector.dropZoneId,
    };
  }
  const point = options.screenPoint?.();
  if (
    point === undefined ||
    !Number.isSafeInteger(point.x) ||
    !Number.isSafeInteger(point.y)
  ) {
    throw new RangeError("drop screen point must use integer screen DIPs");
  }
  return {
    kind: "screen_point",
    point: { x: point.x, y: point.y },
  };
}

function commitResult(response: PanelTransferResponse): DragDropCommitResult {
  if (response.status === "committed") return { status: "committed" };
  return { status: "rejected", reason: response.abort.source.code };
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

export type { CrossWindowDragReceipt };
