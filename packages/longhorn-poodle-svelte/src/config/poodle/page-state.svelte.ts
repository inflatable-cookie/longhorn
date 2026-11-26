import { onMount } from "svelte";

import { CONFIG_OPERATIONS_PROTOCOL_VERSION } from "@inflatable-cookie/longhorn/config";
import type { ConfigOperationsSnapshot } from "@inflatable-cookie/longhorn/config";
import { errorMessage } from "../../failures.ts";
import type { ConfigOperationsPageProps } from "./types.ts";

type Activity = "working" | "loading" | "inspecting" | "planning" | "publishing" | "recovering";

export function useConfigPage(
  initialSnapshot: ConfigOperationsSnapshot | null,
  readSnapshot: () => Promise<ConfigOperationsSnapshot>,
  onSnapshot: ConfigOperationsPageProps["onSnapshot"],
) {
  let snapshot = $state.raw(initialSnapshot);
  let activity = $state<Activity | "idle">("idle");
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);

  function acceptSnapshot(value: ConfigOperationsSnapshot): void {
    snapshot = value;
    onSnapshot?.(value);
  }

  async function run(next: Activity, action: () => Promise<void>): Promise<void> {
    if (activity !== "idle") {
      return;
    }
    activity = next;
    error = null;
    notice = null;
    try {
      await action();
    } catch (error_) {
      error = errorMessage(error_);
    } finally {
      activity = "idle";
    }
  }

  async function refresh(): Promise<void> {
    await run("loading", async () => acceptSnapshot(await readSnapshot()));
  }

  onMount(() => {
    if (snapshot === null) {
      void refresh();
    }
  });

  return {
    get snapshot() {
      return snapshot;
    },
    get activity() {
      return activity;
    },
    get busy() {
      return activity !== "idle";
    },
    get error() {
      return error;
    },
    set error(message: string | null) {
      error = message;
    },
    get notice() {
      return notice;
    },
    set notice(message: string | null) {
      notice = message;
    },
    acceptSnapshot,
    run,
    refresh,
  };
}

export function configRequest(nextRequestId: () => string) {
  return { protocolVersion: CONFIG_OPERATIONS_PROTOCOL_VERSION, requestId: nextRequestId() };
}
