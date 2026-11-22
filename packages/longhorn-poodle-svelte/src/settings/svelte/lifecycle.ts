import { onMount } from "svelte";

import { reportFailure } from "../../failures.ts";

import type { SettingsSession } from "./session.svelte.ts";
import type { SettingsRendererResolver } from "./types.ts";

export function useSettingsSession(
  getSession: () => SettingsSession,
  getRendererResolver: () => SettingsRendererResolver,
): void {
  onMount(() => {
    const session = getSession();
    const rendererResolver = getRendererResolver();
    void session.start(rendererResolver).catch(() => undefined);
    return () => {
      void session.stop().catch(reportFailure);
    };
  });
}
