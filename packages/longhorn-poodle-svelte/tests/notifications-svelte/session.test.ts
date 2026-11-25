import { fireEvent, render, waitFor, within } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

import { NotificationToastHost } from "../../src/notifications/poodle.ts";
import NotificationHarness from "./NotificationHarness.svelte";
import { NotificationSession } from "../../src/notifications/svelte.ts";
import { createMountedSession, MountedNotificationPort, withNewRecord } from "./support.ts";

describe("notification Svelte and public Poodle adapters", () => {
  it("keeps notification authority behind commands and observation", async () => {
    const { session } = createMountedSession();
    await session.start();
    expect(Reflect.has(session, "controller")).toBe(false);
    expect(Reflect.has(session, "unobserve")).toBe(false);
    expect(Reflect.set(session, "selectedNotificationId", "notification:scan")).toBe(false);
    expect(Object.isFrozen(session.records[0]?.draft)).toBe(true);

    const observed = vi.fn();
    const unobserve = session.observe(observed);
    session.select("notification:scan");
    expect(session.selected?.notificationId).toBe("notification:scan");
    expect(observed).toHaveBeenCalledTimes(1);
    unobserve();
    await session.dispose();
    expect(observed).toHaveBeenCalledTimes(1);
  });

  it("moves toast presentation and dismissal to the replacement session", async () => {
    const first = createMountedSession();
    const second = createMountedSession();
    await Promise.all([first.session.start(), second.session.start()]);
    first.port.publish(withNewRecord(first.port.snapshotValue));
    const mounted = render(NotificationToastHost, {
      props: { session: first.session, autoDismissMs: 0 },
    });
    await mounted.findByText("New notification");

    const replacement = withNewRecord(second.port.snapshotValue);
    const record = replacement.page.records[0];
    if (record === undefined) {
      throw new Error("missing notification record");
    }
    record.draft.title = "Replacement notification";
    second.port.publish(replacement);
    await waitFor(() => expect(second.session.toasts).toHaveLength(1));
    await mounted.rerender({ session: second.session });

    expect(mounted.queryByText("New notification")).toBeNull();
    expect(await mounted.findByText("Replacement notification")).toBeTruthy();
    await fireEvent.click(mounted.getByRole("button", { name: /dismiss/i }));
    expect(second.session.toasts).toHaveLength(0);
    expect(first.session.toasts).toHaveLength(1);
    mounted.unmount();
    await Promise.all([first.session.dispose(), second.session.dispose()]);
  });

  it("expires only transient toast presentation", async () => {
    vi.useFakeTimers();
    const state = createMountedSession();
    const mounted = render(NotificationHarness, {
      props: { session: state.session, autoDismissMs: 100 },
    });
    await vi.waitFor(() => expect(state.session.status.kind).toBe("ready"));
    state.port.publish(withNewRecord(state.port.snapshotValue));
    await vi.waitFor(() => expect(state.session.toasts).toHaveLength(1));
    expect(mounted.getAllByText("New notification")).toHaveLength(2);

    await vi.advanceTimersByTimeAsync(100);
    await vi.waitFor(() => expect(state.session.toasts).toHaveLength(0));
    expect(mounted.getByRole("button", { name: "New notification" })).toBeTruthy();
    expect(state.session.records.map((record) => record.notificationId)).toContain(
      "notification:new",
    );
    await Promise.resolve(mounted.unmount());
    vi.useRealTimers();
  });

  it("remounts retained authority without replaying old toasts", async () => {
    const state = createMountedSession();
    const first = render(NotificationHarness, { props: { session: state.session } });
    await waitFor(() => expect(state.session.status.kind).toBe("ready"));
    state.port.publish(withNewRecord(state.port.snapshotValue));
    await waitFor(() => expect(state.session.toasts).toHaveLength(1));
    await Promise.resolve(first.unmount());
    await waitFor(() => expect(state.port.listeners.size).toBe(0));

    const second = render(NotificationHarness, { props: { session: state.session } });
    await second.findByRole("button", { name: "New notification" });
    expect(state.session.toasts).toEqual([]);
    expect(second.container.querySelector(".poodle-toast-host")).toBeNull();
    await Promise.resolve(second.unmount());
    expect(state.port.unlistenCount).toBe(2);
  });

  it("keeps selection isolated across two sessions on one authority", async () => {
    const port = new MountedNotificationPort();
    const firstSession = new NotificationSession({ port });
    const secondSession = new NotificationSession({ port });
    const first = render(NotificationHarness, { props: { session: firstSession } });
    const second = render(NotificationHarness, { props: { session: secondSession } });
    await waitFor(() => {
      expect(port.listeners.size).toBe(2);
      expect(firstSession.status.kind).toBe("ready");
      expect(secondSession.status.kind).toBe("ready");
    });
    await fireEvent.click(within(first.container).getByRole("button", { name: "Scan complete" }));
    expect(firstSession.selectedNotificationId).toBe("notification:scan");
    expect(secondSession.selectedNotificationId).toBeUndefined();
    await Promise.resolve(first.unmount());
    await Promise.resolve(second.unmount());
  });

  it("announces a failed older page and lets the reader retry", async () => {
    const state = createMountedSession();
    state.port.snapshotValue.page.hasMore = true;
    let rejectPage!: (error: Error) => void;
    state.port.pageGate = new Promise<void>((_resolve, reject) => {
      rejectPage = reject;
    });
    const mounted = render(NotificationHarness, { props: { session: state.session } });
    await waitFor(() => expect(state.session.status.kind).toBe("ready"));

    await fireEvent.click(mounted.getByRole("button", { name: "Load older" }));
    await waitFor(() => expect(state.session.pagination.kind).toBe("loading"));
    expect(mounted.getByRole("button", { name: "Load older" }).hasAttribute("disabled")).toBe(true);
    state.port.pageGate = undefined;
    rejectPage(new Error("Page request failed"));
    await waitFor(() => expect(state.session.pagination.kind).toBe("failed"));
    expect(mounted.getByText("Page request failed")).toBeTruthy();

    await fireEvent.click(mounted.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(state.session.pagination.kind).toBe("idle"));
    expect(mounted.queryByText("Page request failed")).toBeNull();
    await Promise.resolve(mounted.unmount());
  });
});
