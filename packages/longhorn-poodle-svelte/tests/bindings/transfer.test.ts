import { render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { EventTransport } from "@inflatable-cookie/longhorn/core";
import {
  TRANSFER_CANCEL_COMMAND,
  TRANSFER_CLIENT_CHANGED_EVENT,
  TRANSFER_COMMIT_PANEL_COMMAND,
  TRANSFER_PUBLISH_LEASE_COMMAND,
  TRANSFER_SNAPSHOT_COMMAND,
  TRANSFER_START_PANEL_COMMAND,
  TransferClient,
} from "@inflatable-cookie/longhorn/transfer";
import type {
  PanelSessionStartRequest,
  PanelTransferCommand,
  PanelTransferResponse,
  TransferCancelRequest,
  TransferLeaseRequest,
  TransferLeaseResponse,
  TransferSessionResponse,
} from "@inflatable-cookie/longhorn/transfer";

import fixture from "../../../../fixtures/transfer/protocol-v1.json";
import {
  LeaseReleaseRejectedError,
  LeaseReleaseUnavailableError,
  TransferState,
} from "../../src/bindings/transfer.svelte.ts";
import type { TimerScheduler, TransferStateOptions } from "../../src/bindings/transfer.svelte.ts";
import LifecycleHarness from "./LifecycleHarness.svelte";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolve_) => {
    resolve = resolve_;
  });
  return { promise, resolve };
}

describe("TransferState", () => {
  it("cleans late preparation, timer, lease, and listener on stop", async () => {
    const preparation = deferred<TransferSessionResponse>();
    const timers = new Set<unknown>();
    const scheduler: TimerScheduler = {
      set(_delay, _callback) {
        const handle = {};
        timers.add(handle);
        return handle;
      },
      clear(handle) {
        timers.delete(handle);
      },
    };
    let unlistens = 0;
    let cancellations = 0;
    let leaseReleases = 0;
    const snapshot = fixture.client_snapshot;
    const transport: EventTransport = {
      async listen(event) {
        expect(event).toBe(TRANSFER_CLIENT_CHANGED_EVENT);
        return () => {
          unlistens += 1;
        };
      },
      async invoke(command, arguments_) {
        if (command === TRANSFER_SNAPSHOT_COMMAND) {
          return snapshot;
        }
        if (command === TRANSFER_START_PANEL_COMMAND) {
          return preparation.promise;
        }
        if (command === TRANSFER_CANCEL_COMMAND) {
          cancellations += 1;
          const request = arguments_.request as TransferCancelRequest;
          return {
            status: "cancelled",
            cancellation: {
              protocol_version: 1,
              request_id: request.request_id,
              session_id: request.session_id,
              status: "cancelled",
            },
          };
        }
        if (command === TRANSFER_PUBLISH_LEASE_COMMAND) {
          const request = arguments_.request as TransferLeaseRequest;
          if (request.zones.length === 0) {
            leaseReleases += 1;
          }
          return {
            status: "published",
            lease: {
              protocol_version: 1,
              request_id: request.request_id,
              client_id: request.client_id,
              client_epoch: request.client_epoch,
              generation: request.generation,
              zone_count: request.zones.length,
            },
          } satisfies TransferLeaseResponse;
        }
        throw new Error(`unexpected command: ${command}`);
      },
    };

    let cancellationSequence = 0;
    const state = new TransferState({
      client: new TransferClient(transport),
      preparationTimeoutMs: 500,
      scheduler,
      makeCancellationRequest: (sessionId) => ({
        protocol_version: 1,
        request_id: `cancel:${++cancellationSequence}`,
        session_id: sessionId,
      }),
      makeLeaseReleaseRequest: (current, lease) => ({
        protocol_version: 1,
        request_id: "lease:release",
        client_id: current.client_id,
        client_epoch: current.client_epoch,
        generation: lease.generation + 1,
        zones: [],
      }),
    });
    const mounted = render(LifecycleHarness, { props: { state } });
    await state.start();

    const prepareTask = state.preparePanel(fixture.session_requests[0] as PanelSessionStartRequest);
    await state.publishLease(fixture.lease_requests[0] as TransferLeaseRequest);
    await Promise.resolve(mounted.unmount());
    const stopTask = state.stop();
    preparation.resolve(fixture.session_responses[0] as TransferSessionResponse);
    await Promise.all([prepareTask, stopTask]);

    expect(timers.size).toBe(0);
    expect(cancellations).toBe(1);
    expect(leaseReleases).toBe(1);
    expect(unlistens).toBe(1);
    expect(state.status.kind).toBe("idle");
    expect(state.preparation.status).toBe("idle");
    expect(state.lease.status).toBe("idle");
  });

  it.each(["unavailable", "rejected", "cancel failed"] as const)(
    "finishes teardown and destroys the state when cleanup is %s",
    async (failureMode) => {
      let unlistens = 0;
      let cancellations = 0;
      let releases = 0;
      const cancelFailure = new Error("cancel failed");
      const transport: EventTransport = {
        async listen() {
          return () => {
            unlistens += 1;
          };
        },
        async invoke(command, arguments_) {
          switch (command) {
            case TRANSFER_SNAPSHOT_COMMAND:
              return fixture.client_snapshot;
            case TRANSFER_START_PANEL_COMMAND:
              return fixture.session_responses[0];
            case TRANSFER_CANCEL_COMMAND:
              cancellations += 1;
              if (failureMode === "cancel failed") {
                throw cancelFailure;
              }
              return fixture.cancel_responses[0];
            case TRANSFER_PUBLISH_LEASE_COMMAND: {
              const request = arguments_.request as TransferLeaseRequest;
              if (request.zones.length === 0) {
                releases += 1;
                if (failureMode === "rejected") {
                  return { status: "aborted", abort: fixture.aborts[0] };
                }
                return {
                  status: "published",
                  lease: {
                    protocol_version: 1,
                    request_id: request.request_id,
                    client_id: request.client_id,
                    client_epoch: request.client_epoch,
                    generation: request.generation,
                    zone_count: 0,
                  },
                };
              }
              return fixture.lease_responses[0];
            }
            default:
              throw new Error(`unexpected command: ${command}`);
          }
        },
      };
      const options: TransferStateOptions = {
        client: new TransferClient(transport),
        makeCancellationRequest: (sessionId) => ({
          protocol_version: 1,
          request_id: "cancel:stop",
          session_id: sessionId,
        }),
      };
      const state = new TransferState(
        failureMode === "unavailable"
          ? options
          : {
              ...options,
              makeLeaseReleaseRequest: (current, lease) => ({
                protocol_version: 1,
                request_id: "lease:release",
                client_id: current.client_id,
                client_epoch: current.client_epoch,
                generation: lease.generation + 1,
                zones: [],
              }),
            },
      );

      await state.start();
      await state.preparePanel(fixture.session_requests[0] as PanelSessionStartRequest);
      await state.publishLease(fixture.lease_requests[0] as TransferLeaseRequest);
      const result = expect(state.destroy()).rejects;
      if (failureMode === "cancel failed") {
        await result.toBe(cancelFailure);
      } else {
        await result.toBeInstanceOf(
          failureMode === "unavailable" ? LeaseReleaseUnavailableError : LeaseReleaseRejectedError,
        );
      }
      expect(cancellations).toBe(1);
      expect(releases).toBe(failureMode === "unavailable" ? 0 : 1);
      expect(unlistens).toBe(1);
      expect(state.status.kind).toBe("idle");
      expect(state.lease.status).toBe("idle");
      expect(state.cancellation.status).toBe("idle");
      await state.stop();
      await expect(state.start()).rejects.toThrow(/destroyed/);
    },
  );

  it.each([-1, Number.POSITIVE_INFINITY, Number.NaN])(
    "rejects invalid preparation timeout %s before entering preparing",
    async (preparationTimeoutMs) => {
      const transport: EventTransport = {
        async listen() {
          return () => undefined;
        },
        async invoke(command) {
          if (command === TRANSFER_SNAPSHOT_COMMAND) {
            return fixture.client_snapshot;
          }
          throw new Error(`unexpected command: ${command}`);
        },
      };
      const state = new TransferState({
        client: new TransferClient(transport),
        preparationTimeoutMs,
        makeCancellationRequest: (sessionId) => ({
          protocol_version: 1,
          request_id: "cancel:timeout",
          session_id: sessionId,
        }),
      });
      await state.start();
      await expect(
        state.preparePanel(fixture.session_requests[0] as PanelSessionStartRequest),
      ).rejects.toBeInstanceOf(RangeError);
      expect(state.preparation.status).toBe("idle");
      expect(state.status.kind).toBe("ready");
      await state.stop();
    },
  );

  it("exposes explicit cancellation and authoritative completion state", async () => {
    const transport: EventTransport = {
      async listen() {
        return () => undefined;
      },
      async invoke(command, arguments_) {
        if (command === TRANSFER_SNAPSHOT_COMMAND) {
          return fixture.client_snapshot;
        }
        if (command === TRANSFER_CANCEL_COMMAND) {
          const request = arguments_.request as TransferCancelRequest;
          return {
            status: "cancelled",
            cancellation: {
              protocol_version: 1,
              request_id: request.request_id,
              session_id: request.session_id,
              status: "cancelled",
            },
          };
        }
        if (command === TRANSFER_COMMIT_PANEL_COMMAND) {
          return fixture.commit_responses[0];
        }
        throw new Error(`unexpected command: ${command}`);
      },
    };
    const state = new TransferState({
      client: new TransferClient(transport),
      makeCancellationRequest: (sessionId) => ({
        protocol_version: 1,
        request_id: "cancel:auto",
        session_id: sessionId,
      }),
    });
    await state.start();

    await state.cancel(fixture.cancel_requests[0] as TransferCancelRequest);
    expect(state.cancellation.status).toBe("cancelled");

    const response = await state.commitPanel(fixture.commit_requests[0] as PanelTransferCommand);
    expect(response).toEqual(fixture.commit_responses[0] as PanelTransferResponse);
    expect(state.completion.status).toBe("committed");
    if (state.completion.status === "committed") {
      expect(state.completion.response.completion.authoritative_document.revision).toBe(8);
    }
    await state.destroy();
  });
});
