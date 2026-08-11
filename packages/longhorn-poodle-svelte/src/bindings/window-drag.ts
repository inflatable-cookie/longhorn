const INTERACTIVE_SELECTOR = [
  "button",
  "a",
  "input",
  "textarea",
  "select",
  "[role='button']",
  "[data-no-window-drag]",
].join(",");

export interface WindowDragOptions {
  readonly startDragging: () => void | Promise<void>;
  readonly reportError: (error: unknown) => void;
  /**
   * Double-clicking the chrome zooms the window, the platform convention for a
   * titlebar. Omit it and a double click does nothing, which is what a surface
   * that is a drag handle but not a titlebar wants.
   *
   * macOS lets the operator remap this gesture to minimise or to nothing at
   * all, and no Tauri API reports that preference, so this cannot honour it.
   * Zoom is the default and the only setting most operators ever see; a host
   * that knows better can pass its own function.
   */
  readonly toggleMaximize?: () => void | Promise<void>;
}

export interface WindowDragAction {
  update(options: WindowDragOptions): void;
  destroy(): void;
}

export function windowDrag(
  node: HTMLElement,
  initialOptions: WindowDragOptions,
): WindowDragAction {
  let options = initialOptions;

  // The same eligibility test for both gestures: primary button, no modifiers,
  // and not on something the operator meant to click.
  function isChromeGesture(event: MouseEvent): boolean {
    return (
      event.button === 0 &&
      !event.altKey &&
      !event.ctrlKey &&
      !event.metaKey &&
      !event.shiftKey &&
      event.target instanceof Element &&
      !event.target.closest(INTERACTIVE_SELECTOR)
    );
  }

  function invoke(action: () => void | Promise<void>): void {
    try {
      void Promise.resolve(action()).catch(report);
    } catch (error) {
      report(error);
    }
  }

  function handleMouseDown(event: MouseEvent): void {
    if (!isChromeGesture(event)) return;
    event.preventDefault();
    invoke(options.startDragging);
  }

  function handleDoubleClick(event: MouseEvent): void {
    const { toggleMaximize } = options;
    if (!toggleMaximize || !isChromeGesture(event)) return;
    event.preventDefault();
    invoke(toggleMaximize);
  }

  function report(error: unknown): void {
    try {
      options.reportError(error);
    } catch {
      // Reporting failure must not become an unhandled drag failure.
    }
  }

  // Both are registered unconditionally. `toggleMaximize` is read at dispatch,
  // so `update()` can add or remove it without rebinding, which is what a
  // Svelte action's reactive options are for.
  node.addEventListener("mousedown", handleMouseDown);
  node.addEventListener("dblclick", handleDoubleClick);
  return {
    update(nextOptions) {
      options = nextOptions;
    },
    destroy() {
      node.removeEventListener("mousedown", handleMouseDown);
      node.removeEventListener("dblclick", handleDoubleClick);
    },
  };
}
