import { render, within } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

import {
  NotificationSession,
} from "@longhorn/notifications/svelte";
import type {
  NotificationActionInvocation,
  NotificationChangedEvent,
  NotificationMutationCommand,
  NotificationSnapshot,
  NotificationSnapshotQuery,
} from "@longhorn/notifications";
import { OperationSession } from "@longhorn/operation/svelte";
import type {
  OperationCancellationCommand,
  OperationMutationCommand,
  OperationSnapshotQuery,
} from "@longhorn/operation";

import ActivityHarness from "./ActivityHarness.svelte";
import fixtureValue from "./fixture.json";

const fixture = fixtureValue as any;
const clone = <Value>(value: Value): Value => JSON.parse(JSON.stringify(value)) as Value;

describe("packed Loophole activity composition", () => {
  it("isolates windows, expires only transient toasts, and remounts retained truth", async () => {
    vi.useFakeTimers();
    const operationSnapshot = clone(fixture.operation.results.at(-1).snapshot);
    const notificationSnapshot = clone(
      fixture.notifications.results.at(-1).snapshot,
    ) as NotificationSnapshot;
    const notificationListeners = new Set<(event: unknown) => void>();
    let notificationUnlistenCount = 0;
    let admitted = true;
    const actionDecisions: boolean[] = [];

    const makeSessions = () => ({
      operations: new OperationSession({
        port: {
          snapshot: async (query: OperationSnapshotQuery) => ({
            requestId: query.requestId,
            snapshot: clone(operationSnapshot),
          }),
          mutate: async (_command: OperationMutationCommand) => {
            throw new Error("not expected");
          },
          cancel: async (_command: OperationCancellationCommand) => {
            throw new Error("not expected");
          },
          listen: () => () => {},
          nextRequestId: () => "request:mounted-operation",
        },
      }),
      notifications: new NotificationSession({
        port: {
          snapshot: async (query: NotificationSnapshotQuery) => ({
            requestId: query.requestId,
            snapshot: clone(notificationSnapshot),
          }),
          mutate: async (_command: NotificationMutationCommand) => {
            throw new Error("not expected");
          },
          listen: (listener) => {
            notificationListeners.add(listener);
            return () => {
              if (notificationListeners.delete(listener)) notificationUnlistenCount += 1;
            };
          },
          nextRequestId: () => "request:mounted-notification",
        },
        toast: { select: () => true },
        actions: {
          admitAndExecute: async (_invocation: NotificationActionInvocation) => {
            actionDecisions.push(admitted);
            if (!admitted) throw new Error("action no longer admitted");
          },
        },
      }),
    });

    const first = makeSessions();
    const second = makeSessions();
    const firstWindow = render(ActivityHarness, { props: first });
    const secondWindow = render(ActivityHarness, { props: second });
    await vi.waitFor(() => {
      expect(first.operations.status.kind).toBe("ready");
      expect(first.notifications.status.kind).toBe("ready");
      expect(second.operations.status.kind).toBe("ready");
      expect(second.notifications.status.kind).toBe("ready");
    });
    expect(notificationListeners.size).toBe(2);
    expect(first.notifications).not.toBe(second.notifications);
    const firstView = within(firstWindow.container);
    expect(firstView.getByRole("button", { name: "Render final sequence" })).toBeTruthy();
    expect(firstView.getByRole("button", { name: "Render complete" })).toBeTruthy();

    const previous = notificationSnapshot.ledgerRevision;
    const actionRecord = notificationSnapshot.page.records.find(
      (record) => record.draft.actions.length > 0,
    );
    expect(actionRecord).toBeDefined();
    const fresh = clone(actionRecord!);
    notificationSnapshot.ledgerRevision += 1;
    fresh.notificationId = "notification:device-loss";
    fresh.sequence += 1;
    fresh.lastChangedLedgerRevision = notificationSnapshot.ledgerRevision;
    fresh.draft.title = "Audio device disconnected";
    notificationSnapshot.page.records.unshift(fresh);
    notificationSnapshot.page.totalCount += 1;
    notificationSnapshot.retainedCount += 1;
    notificationSnapshot.unseenCount += 1;
    const event: NotificationChangedEvent = {
      protocolVersion: 1,
      requestId: "request:device-loss",
      authority: notificationSnapshot.authority,
      previousLedgerRevision: previous,
      committedLedgerRevision: notificationSnapshot.ledgerRevision,
      affectedNotificationIds: [fresh.notificationId],
      kind: "added",
    };
    for (const listener of notificationListeners) listener(event);
    await vi.waitFor(() => {
      expect(first.notifications.toasts).toHaveLength(1);
      expect(second.notifications.toasts).toHaveLength(1);
    });
    expect(firstView.getAllByText("Audio device disconnected")).toHaveLength(2);

    const action = fresh.draft.actions[0];
    expect(action).toBeDefined();
    admitted = false;
    await expect(
      first.notifications.invokeAction(fresh.notificationId, action!.referenceId),
    ).rejects.toThrow("action no longer admitted");
    expect(actionDecisions).toEqual([false]);

    await vi.advanceTimersByTimeAsync(1000);
    await vi.waitFor(() => {
      expect(first.notifications.toasts).toHaveLength(0);
      expect(second.notifications.toasts).toHaveLength(0);
    });
    expect(firstView.getByRole("button", { name: "Audio device disconnected" })).toBeTruthy();

    await Promise.all([firstWindow.unmount(), secondWindow.unmount()]);
    await vi.waitFor(() => expect(notificationListeners.size).toBe(0));
    expect(notificationUnlistenCount).toBe(2);

    const remounted = makeSessions();
    const remountedWindow = render(ActivityHarness, { props: remounted });
    await vi.waitFor(() => expect(remounted.notifications.status.kind).toBe("ready"));
    expect(remounted.notifications.toasts).toEqual([]);
    expect(within(remountedWindow.container).getByRole(
      "button",
      { name: "Audio device disconnected" },
    )).toBeTruthy();
    await remountedWindow.unmount();
    await vi.waitFor(() => expect(notificationListeners.size).toBe(0));
    expect(notificationUnlistenCount).toBe(3);
    vi.useRealTimers();
  });
});
