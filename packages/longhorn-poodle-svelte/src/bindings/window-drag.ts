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

  function handleMouseDown(event: MouseEvent): void {
    if (
      event.button !== 0 ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      event.shiftKey ||
      !(event.target instanceof Element) ||
      event.target.closest(INTERACTIVE_SELECTOR)
    ) {
      return;
    }

    event.preventDefault();
    try {
      void Promise.resolve(options.startDragging()).catch(report);
    } catch (error) {
      report(error);
    }
  }

  function report(error: unknown): void {
    try {
      options.reportError(error);
    } catch {
      // Reporting failure must not become an unhandled drag failure.
    }
  }

  node.addEventListener("mousedown", handleMouseDown);
  return {
    update(nextOptions) {
      options = nextOptions;
    },
    destroy() {
      node.removeEventListener("mousedown", handleMouseDown);
    },
  };
}
