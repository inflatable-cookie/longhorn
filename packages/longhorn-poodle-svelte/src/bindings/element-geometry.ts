/** Client rect origins can move when the window or a nested scroller scrolls. */
export function observeViewportMovement(node: HTMLElement, changed: () => void): () => void {
  const view = node.ownerDocument.defaultView;
  view?.addEventListener("scroll", changed, { capture: true, passive: true });
  view?.addEventListener("resize", changed);
  return () => {
    view?.removeEventListener("scroll", changed, true);
    view?.removeEventListener("resize", changed);
  };
}
