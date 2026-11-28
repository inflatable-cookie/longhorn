import { notify, reportError, resolveDropSelector } from "../bindings/drop-target.ts";
import type { DropSelector } from "../bindings/drop-target.ts";
import { errorMessage } from "../failures.ts";

import {
  CROSS_WINDOW_DRAG_PROTOCOL_VERSION,
  decodeDockPanelSubject,
} from "@inflatable-cookie/poodle-core";
import type {
  CrossWindowDragReceipt,
  CrossWindowDragSourceBridge,
  CrossWindowDragTargetBridge,
  DragDropCommitResult,
} from "@inflatable-cookie/poodle-core";
import { TRANSFER_PROTOCOL_VERSION } from "@inflatable-cookie/longhorn/transfer";
import type {
  PanelSessionStartRequest,
  PanelTransferCommand,
  PanelTransferResponse,
  TransferSessionResponse,
} from "@inflatable-cookie/longhorn/transfer";
import type { TransferState } from "@inflatable-cookie/longhorn-poodle-svelte/transfer";

export type PanelDropSelector = DropSelector;

export interface PanelTransferDragSourceOptions {
  readonly state: TransferState;
  readonly makeStartRequest: (panelInstanceId: string) => PanelSessionStartRequest;
  readonly reportError: (error: unknown) => void;
  readonly onPreparation?: (response: TransferSessionResponse) => void;
}

export interface PanelTransferDropTargetOptions {
  readonly state: TransferState;
  readonly selector: PanelDropSelector;
  /** Host-owned projection and terminal events for incoming drags. */
  readonly subscribeToIncoming?: CrossWindowDragTargetBridge["subscribe"];
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
      if (signal.aborted) {
        return null;
      }
      const decoded = decodeDockPanelSubject(request.subject.id);
      if (decoded === null) {
        return null;
      }

      const startRequest = options.makeStartRequest(decoded.panelId);
      const response = await options.state.preparePanel(startRequest);
      notify(options.onPreparation, response, options.reportError);
      if (response.status !== "started") {
        return null;
      }

      return receiptFor(response.session.payload.session_id);
    },
    start() {
      return () => undefined;
    },
    cancel() {
      void options.state.cancelPreparation().catch((error: unknown) => {
        reportError(options.reportError, error);
      });
    },
  };
}

export function createPanelTransferDropTarget(
  options: PanelTransferDropTargetOptions,
): CrossWindowDragTargetBridge {
  return {
    capabilities: {
      pointer: true,
      touch: false,
      keyboardTargetPicker: false,
    },
    subscribe(listener) {
      return options.subscribeToIncoming?.(listener) ?? (() => undefined);
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
          selector: resolveDropSelector(options.selector, options.screenPoint),
        };
        const response = await options.state.commitPanel(transferRequest);
        notify(options.onResponse, response, options.reportError);
        return commitResult(response);
      } catch (error) {
        reportError(options.reportError, error);
        return {
          status: "failed",
          reason: errorMessage(error),
        };
      } finally {
        notify(options.onTerminal, undefined, options.reportError);
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

function commitResult(response: PanelTransferResponse): DragDropCommitResult {
  if (response.status === "committed") {
    return { status: "committed" };
  }
  return { status: "rejected", reason: response.abort.source.code };
}

export type { CrossWindowDragReceipt };
