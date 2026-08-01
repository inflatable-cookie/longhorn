import type { NativeContentSession } from "./session.svelte.ts";

export interface NativeContentViewportAction {
  update(session: NativeContentSession): void;
  destroy(): void;
}

/** Binds the exact consumer-owned element used as the native viewport. */
export function nativeContentViewport(
  node: HTMLElement,
  session: NativeContentSession,
): NativeContentViewportAction {
  let current = session;
  let unbind = current.bindViewport(node);

  return {
    update(next) {
      if (next === current) return;
      unbind();
      current = next;
      unbind = current.bindViewport(node);
    },
    destroy() {
      unbind();
    },
  };
}
