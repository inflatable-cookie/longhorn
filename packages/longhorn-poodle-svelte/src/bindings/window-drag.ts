// The whole titlebar gesture set: drag to move, double click to zoom.
//
// This deliberately replaces Tauri's `data-tauri-drag-region`, and the two must
// not both be applied to one element. Tauri's injected handler reads the
// attribute off the event target only, so it never checks modifiers and never
// looks at ancestors; leaving the attribute in place alongside this action
// means Tauri answers the gesture first, `start_dragging` is invoked twice, and
// the modifier and interactive-descendant exclusions below are all defeated.
//
// What this adds over the attribute is the exclusions: a header drags as a
// whole except where the operator meant to click. Tauri's version needs the
// attribute placed on each draggable element, which is why consumers grew
// dedicated `aria-hidden` spacer divs to drag by.
//
// What it owes the attribute is macOS fidelity, and the zoom below is ported
// from Tauri's `drag.js` rather than reinvented. See tauri-apps/tauri#8306:
// on macOS the zoom lands on pointer up and is cancelled if the pointer moved,
// so an accidental double click can be taken back. Elsewhere it lands on the
// second press.

const INTERACTIVE_SELECTOR = [
  "button",
  "a",
  "input",
  "textarea",
  "select",
  "[role='button']",
  "[data-no-window-drag]",
].join(",");

/** Matches the platform vocabulary the command domain already generates. */
export type WindowDragPlatform = "macOs" | "windows" | "linux";

/**
 * Zoom is configured as a unit so the platform cannot be forgotten.
 *
 * A bare `toggleMaximize` would need a platform default, and either default is
 * wrong somewhere: assume macOS and Windows zooms on an unmovable gesture,
 * assume otherwise and macOS loses the cancel its operators expect.
 */
export interface WindowZoomOptions {
  readonly toggleMaximize: () => void | Promise<void>;
  readonly platform: WindowDragPlatform;
}

export interface WindowDragOptions {
  readonly startDragging: () => void | Promise<void>;
  readonly reportError: (error: unknown) => void;
  /**
   * Omit to leave a double click inert, which is what a drag handle that is not
   * a titlebar wants.
   *
   * macOS lets the operator remap this gesture to minimise or to nothing at
   * all, and no Tauri API reports that preference, so this cannot honour it.
   * Zoom is the default and the setting most operators never change; a host
   * that knows better passes its own function.
   */
  readonly zoom?: WindowZoomOptions;
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
  // Where the deferred macOS zoom was pressed. Null whenever none is pending.
  let pendingZoomAt: { x: number; y: number } | null = null;

  // One eligibility test for every gesture: primary button, no modifiers, and
  // not on something the operator meant to click.
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

  function report(error: unknown): void {
    try {
      options.reportError(error);
    } catch {
      // Reporting failure must not become an unhandled drag failure.
    }
  }

  function handleMouseDown(event: MouseEvent): void {
    pendingZoomAt = null;
    if (!isChromeGesture(event)) return;

    // `detail` counts clicks in the sequence, so the second press of a double
    // click is a zoom rather than the start of another drag.
    const isSecondPress = event.detail === 2;
    const { zoom } = options;

    if (isSecondPress) {
      if (!zoom) return;
      event.preventDefault();
      if (zoom.platform === "macOs") {
        pendingZoomAt = { x: event.clientX, y: event.clientY };
        return;
      }
      invoke(zoom.toggleMaximize);
      return;
    }

    event.preventDefault();
    invoke(options.startDragging);
  }

  function handleMouseUp(event: MouseEvent): void {
    const at = pendingZoomAt;
    pendingZoomAt = null;
    const { zoom } = options;
    if (
      !at ||
      !zoom ||
      zoom.platform !== "macOs" ||
      event.button !== 0 ||
      event.detail !== 2 ||
      // Moving between press and release cancels it, which is how macOS lets
      // an accidental double click be taken back.
      event.clientX !== at.x ||
      event.clientY !== at.y
    ) {
      return;
    }
    invoke(zoom.toggleMaximize);
  }

  // Both are registered unconditionally and `zoom` is read at dispatch, so
  // `update()` can add or remove it without rebinding.
  node.addEventListener("mousedown", handleMouseDown);
  node.addEventListener("mouseup", handleMouseUp);
  return {
    update(nextOptions) {
      options = nextOptions;
    },
    destroy() {
      node.removeEventListener("mousedown", handleMouseDown);
      node.removeEventListener("mouseup", handleMouseUp);
    },
  };
}
