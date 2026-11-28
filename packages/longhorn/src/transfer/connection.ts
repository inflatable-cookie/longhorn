import { CheckedSnapshotConnection, CheckedSubscription } from "@inflatable-cookie/longhorn/core";
import type { ConnectionFailureReporter, EventTransport } from "@inflatable-cookie/longhorn/core";

import type { TransferClientSnapshot } from "./generated/protocol.ts";
import { assertValidTransferClientSnapshot } from "./validation.ts";

export const TRANSFER_CLIENT_CHANGED_EVENT = "longhorn://transfer/client-changed";

export type TransferClientSnapshotListener = (snapshot: TransferClientSnapshot) => void;

export class TransferClientConnection extends CheckedSubscription<
  TransferClientSnapshot,
  TransferClientSnapshot
> {
  constructor(
    transport: EventTransport,
    loadSnapshot: () => Promise<TransferClientSnapshot>,
    listener?: TransferClientSnapshotListener,
    onFailure?: ConnectionFailureReporter,
  ) {
    super(
      new CheckedSnapshotConnection({
        listen: (receive) =>
          transport.listen(TRANSFER_CLIENT_CHANGED_EVENT, receive, { scope: "current-webview" }),
        loadSnapshot,
        validateSnapshot: parseTransferClientSnapshot,
        handleEvent: (value) => ({ kind: "snapshot", value }),
        isNewer: isCurrentTransferClientSnapshot,
        onSnapshot: listener,
        onFailure,
        disposedBeforeReadyError: () => new TransferClientConnectionDisposedError(),
      }),
      (snapshot) => {
        if (snapshot === undefined) {
          throw new TransferClientConnectionDisposedError();
        }
        return snapshot;
      },
    );
  }
}

export class TransferClientConnectionDisposedError extends Error {
  constructor() {
    super("transfer client connection was disposed during registration");
    this.name = "TransferClientConnectionDisposedError";
  }
}

function parseTransferClientSnapshot(value: unknown): TransferClientSnapshot {
  assertValidTransferClientSnapshot(value);
  return value;
}

function isCurrentTransferClientSnapshot(
  candidate: TransferClientSnapshot,
  current: TransferClientSnapshot | undefined,
): boolean {
  if (current === undefined || candidate.client_epoch > current.client_epoch) {
    return true;
  }
  return (
    candidate.client_epoch === current.client_epoch && candidate.client_id === current.client_id
  );
}
