import {
  CheckedSnapshotConnection,
  type ConnectionFailure,
  type ConnectionFailureReporter,
  type EventTransport,
} from "@inflatable-cookie/longhorn/core";

import type { TransferClientSnapshot } from "./generated/protocol.ts";
import { assertCompatibleTransferClientSnapshot } from "./validation.ts";

export const TRANSFER_CLIENT_CHANGED_EVENT =
  "longhorn://transfer/client-changed";

export type TransferClientSnapshotListener = (
  snapshot: TransferClientSnapshot,
) => void;

export class TransferClientConnection {
  readonly ready: Promise<TransferClientSnapshot>;

  readonly #connection: CheckedSnapshotConnection<TransferClientSnapshot>;

  constructor(
    transport: EventTransport,
    loadSnapshot: () => Promise<TransferClientSnapshot>,
    listener?: TransferClientSnapshotListener,
    onFailure?: ConnectionFailureReporter,
  ) {
    this.#connection = new CheckedSnapshotConnection({
      listen: (receive) =>
        transport.listen(TRANSFER_CLIENT_CHANGED_EVENT, receive),
      loadSnapshot,
      validateSnapshot: parseTransferClientSnapshot,
      handleEvent: (value) => ({ kind: "snapshot", value }),
      isNewer: isCurrentTransferClientSnapshot,
      onSnapshot: listener,
      onFailure,
      disposedBeforeReadyError: () =>
        new TransferClientConnectionDisposedError(),
    });
    this.ready = this.#connection.ready.then((snapshot) => {
      if (snapshot === undefined) {
        throw new TransferClientConnectionDisposedError();
      }
      return snapshot;
    });
  }

  current(): TransferClientSnapshot | undefined {
    return this.#connection.current();
  }

  failures(): readonly ConnectionFailure[] {
    return this.#connection.failures();
  }

  dispose(): Promise<void> {
    return this.#connection.dispose();
  }
}

export class TransferClientConnectionDisposedError extends Error {
  constructor() {
    super("transfer client connection was disposed during registration");
    this.name = "TransferClientConnectionDisposedError";
  }
}

function parseTransferClientSnapshot(
  value: unknown,
): TransferClientSnapshot {
  assertCompatibleTransferClientSnapshot(value);
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
    candidate.client_epoch === current.client_epoch &&
    candidate.client_id === current.client_id
  );
}
