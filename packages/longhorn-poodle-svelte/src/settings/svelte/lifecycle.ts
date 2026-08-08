import { onMount } from "svelte";

import type { SettingsSession } from "./session.svelte.ts";
import type { SettingsRendererResolver } from "./types.ts";

export function useSettingsSession(
  session: SettingsSession,
  rendererResolver: SettingsRendererResolver,
): void {
  onMount(() => {
    void session.start(rendererResolver).catch(() => undefined);
    return () => {
      void session.stop();
    };
  });
}

