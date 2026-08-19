//! In-page semantic surface for contract 022: tree, live-DOM refs, synthetic
//! input, wait_for predicates, and a bounded page-event ring.
//!
//! Pure page mechanics. No transport, no timers, no animation-frame waits.
//! Refs live as attributes on the elements they name — there is no shim-side
//! table that outlives the DOM. Truncation is a sentinel child with role
//! `truncated` (the core vocabulary has no truncation field; a silent cap
//! is forbidden). Synthetic events are untrusted: `isTrusted` stays false.

export const REF_ATTR = "data-longhorn-agent-ref";
export const REF_SEQ_ATTR = "data-longhorn-agent-ref-seq";
export const TRUNCATED_ROLE = "truncated";
export const TRUNCATED_REF = "truncated";
export const MAX_DEPTH = 24;
export const MAX_NODES = 200;
export const EVENT_BUFFER_LIMIT = 64;
export const SHIM_GLOBAL = "__longhornAgentControl";

const SKIP_TAGS = new Set([
  "SCRIPT",
  "STYLE",
  "NOSCRIPT",
  "TEMPLATE",
  "HEAD",
  "META",
  "LINK",
  "TITLE",
  "IFRAME",
]);

type FormControl = Element & {
  value: string;
  disabled?: boolean;
  checked?: boolean;
  readOnly?: boolean;
  type?: string;
  selectionStart?: number | null;
  selectionEnd?: number | null;
  setSelectionRange?: (start: number, end: number) => void;
  selectedOptions?: ArrayLike<Element>;
  selected?: boolean;
};

function isFormControl(element: Element): element is FormControl {
  const tag = element.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || tag === "OPTION";
}

function prop<T>(element: Element, name: string): T | undefined {
  return (element as unknown as Record<string, T | undefined>)[name];
}

export type ToolErrorBody =
  | { error: "unresolvedRef"; element: string }
  | { error: "unsupported"; message: string };

export type ShimOk<T> = { ok: true } & T;
export type ShimErr = { ok: false; error: ToolErrorBody };
export type ShimResult<T> = ShimOk<T> | ShimErr;
export type ActionOk = ShimOk<Record<string, never>>;

export type SemanticNode = {
  elementRef: string;
  role: string;
  name?: string;
  value?: string;
  states: string[];
  children: SemanticNode[];
};

export type PageState = { url: string; title: string };

export type PageEvent =
  | { seq: number; kind: "console"; level: string; text: string }
  | { seq: number; kind: "error"; message: string }
  | { seq: number; kind: "navigation"; url: string };

export type WaitPredicate =
  | { predicate: "refResolve"; element: string }
  | { predicate: "refAbsent"; element: string }
  | { predicate: "pageUrlContains"; needle: string }
  | { predicate: "pageTitleContains"; needle: string };

export type AgentControlApi = {
  snapshot: () => ShimResult<{ page: PageState; root: SemanticNode }>;
  click: (element: string) => ShimResult<Record<string, never>>;
  type: (element: string, text: string) => ShimResult<Record<string, never>>;
  press: (
    key: string,
    modifiers?: string[],
    element?: string | null,
  ) => ShimResult<Record<string, never>>;
  scroll: (
    deltaX: number,
    deltaY: number,
    element?: string | null,
  ) => ShimResult<Record<string, never>>;
  drag: (source: string, target: string) => ShimResult<Record<string, never>>;
  waitFor: (predicate: WaitPredicate) => ShimResult<{ holds: boolean }>;
  readEvents: (sinceSeq?: number) => {
    events: PageEvent[];
    nextSeq: number;
    dropped: number;
  };
};

export type ShimWorld = {
  document: Document;
  location: { href: string };
  console: Pick<Console, "log" | "info" | "warn" | "error" | "debug">;
  history: History;
  MouseEvent: typeof MouseEvent;
  KeyboardEvent: typeof KeyboardEvent;
  Event: typeof Event;
  InputEvent?: typeof InputEvent;
  addEventListener: (type: string, listener: (event: Event) => void) => void;
  onunhandledrejection?: ((event: Event) => void) | null;
} & Record<string, unknown>;

type Budget = { nodes: number; truncated: boolean };

function unresolved(element: string): ShimResult<never> {
  return { ok: false, error: { error: "unresolvedRef", element } };
}

function ok(): ActionOk {
  return { ok: true } as ActionOk;
}

function resolveRef(document: Document, element: string): Element | null {
  if (element === TRUNCATED_REF || element.length === 0) return null;
  return document.querySelector(`[${REF_ATTR}="${element}"]`);
}

function allocRef(document: Document): string {
  const root = document.documentElement;
  const current = Number(root.getAttribute(REF_SEQ_ATTR) ?? "0");
  const next = Number.isFinite(current) ? current + 1 : 1;
  root.setAttribute(REF_SEQ_ATTR, String(next));
  return `e${next}`;
}

function stamp(element: Element, document: Document): string {
  const existing = element.getAttribute(REF_ATTR);
  if (existing) return existing;
  const id = allocRef(document);
  element.setAttribute(REF_ATTR, id);
  return id;
}

function computedStyle(element: Element): CSSStyleDeclaration | null {
  const view = element.ownerDocument.defaultView;
  return view ? view.getComputedStyle(element) : null;
}

function isVisible(element: Element): boolean {
  if (element.hasAttribute("hidden")) return false;
  if (element.getAttribute("aria-hidden") === "true") return false;
  const style = computedStyle(element);
  if (!style) return true;
  return style.display !== "none" && style.visibility !== "hidden";
}

function implicitRole(element: Element): string | null {
  const tag = element.tagName;
  if (tag === "A") return element.hasAttribute("href") ? "link" : null;
  if (tag === "BUTTON") return "button";
  if (tag === "TEXTAREA") return "textbox";
  if (tag === "SELECT") return "combobox";
  if (tag === "OPTION") return "option";
  if (tag === "SUMMARY") return "button";
  if (tag === "DETAILS") return "group";
  if (tag === "NAV") return "navigation";
  if (tag === "MAIN") return "main";
  if (tag === "HEADER") return "banner";
  if (tag === "FOOTER") return "contentinfo";
  if (tag === "FORM") return "form";
  if (tag === "UL" || tag === "OL") return "list";
  if (tag === "LI") return "listitem";
  if (tag === "TABLE") return "table";
  if (tag === "DIALOG") return "dialog";
  if (tag === "IMG") return "image";
  if (tag === "H1" || tag === "H2" || tag === "H3" || tag === "H4" || tag === "H5" || tag === "H6") {
    return "heading";
  }
  if (tag === "INPUT") {
    const type = (element.getAttribute("type") ?? "text").toLowerCase();
    if (type === "checkbox") return "checkbox";
    if (type === "radio") return "radio";
    if (type === "number") return "spinbutton";
    if (type === "range") return "slider";
    if (type === "button" || type === "submit" || type === "reset" || type === "file") {
      return "button";
    }
    if (type === "hidden") return null;
    return "textbox";
  }
  if (element.hasAttribute("contenteditable")) return "textbox";
  return null;
}

function roleOf(element: Element): string | null {
  const explicit = element.getAttribute("role");
  if (explicit === "none" || explicit === "presentation") return null;
  if (explicit) return explicit;
  return implicitRole(element);
}

function labelledByText(element: Element): string | undefined {
  const ids = element.getAttribute("aria-labelledby");
  if (!ids) return undefined;
  const document = element.ownerDocument;
  const text = ids
    .split(/\s+/)
    .map((id) => document.getElementById(id)?.textContent ?? "")
    .join(" ")
    .replace(/\s+/g, " ")
    .trim();
  return text.length > 0 ? text : undefined;
}

function associatedLabel(element: Element): string | undefined {
  const id = element.id;
  const document = element.ownerDocument;
  if (id) {
    const label = document.querySelector(`label[for="${id}"]`);
    const text = label?.textContent?.replace(/\s+/g, " ").trim();
    if (text) return text;
  }
  const parent = element.closest("label");
  if (parent) {
    const text = parent.textContent?.replace(/\s+/g, " ").trim();
    if (text) return text;
  }
  return undefined;
}

function accessibleName(element: Element): string | undefined {
  const labelled = labelledByText(element);
  if (labelled) return labelled;
  const aria = element.getAttribute("aria-label")?.trim();
  if (aria) return aria;
  const label = associatedLabel(element);
  if (label) return label;
  if (element.tagName === "IMG") {
    const alt = element.getAttribute("alt")?.trim();
    if (alt) return alt;
  }
  const role = roleOf(element);
  if (
    role === "button" ||
    role === "link" ||
    role === "heading" ||
    role === "option" ||
    role === "tab" ||
    role === "listitem"
  ) {
    const text = element.textContent?.replace(/\s+/g, " ").trim();
    if (text) return text;
  }
  const title = element.getAttribute("title")?.trim();
  if (title) return title;
  const placeholder = element.getAttribute("placeholder")?.trim();
  if (placeholder) return placeholder;
  return undefined;
}

function valueOf(element: Element): string | undefined {
  const now = element.getAttribute("aria-valuenow");
  if (now) return now;
  if (element.tagName === "INPUT") {
    const type = (prop<string>(element, "type") ?? "text").toLowerCase();
    if (type === "checkbox" || type === "radio") return undefined;
    return prop<string>(element, "value");
  }
  if (element.tagName === "TEXTAREA") return prop<string>(element, "value");
  if (element.tagName === "SELECT") {
    const selected = prop<ArrayLike<Element>>(element, "selectedOptions");
    const first = selected && selected.length > 0 ? selected[0] : undefined;
    return first?.textContent?.trim() ?? prop<string>(element, "value");
  }
  if (element.hasAttribute("contenteditable")) {
    return element.textContent ?? undefined;
  }
  return undefined;
}

function statesOf(element: Element, document: Document): string[] {
  const states: string[] = [];
  if (isVisible(element)) states.push("visible");
  if (
    element.hasAttribute("disabled") ||
    element.getAttribute("aria-disabled") === "true" ||
    prop<boolean>(element, "disabled") === true
  ) {
    states.push("disabled");
  }
  if (prop<boolean>(element, "checked") === true || element.getAttribute("aria-checked") === "true") {
    states.push("checked");
  }
  if (document.activeElement === element) states.push("focused");
  if (element.getAttribute("aria-expanded") === "true") states.push("expanded");
  if (element.getAttribute("aria-pressed") === "true") states.push("pressed");
  if (
    element.getAttribute("aria-selected") === "true" ||
    prop<boolean>(element, "selected") === true
  ) {
    states.push("selected");
  }
  if (
    element.getAttribute("aria-readonly") === "true" ||
    prop<boolean>(element, "readOnly") === true
  ) {
    states.push("readonly");
  }
  return states;
}

function isInteresting(element: Element): boolean {
  if (SKIP_TAGS.has(element.tagName)) return false;
  // Labels contribute their text to the control's name; they are not nodes.
  if (element.tagName === "LABEL") return false;
  if (roleOf(element)) return true;
  if (accessibleName(element)) return true;
  if (element.tagName === "INPUT") return true;
  if (element.hasAttribute("tabindex")) return true;
  return false;
}

function truncatedNode(): SemanticNode {
  return {
    elementRef: TRUNCATED_REF,
    role: TRUNCATED_ROLE,
    states: [],
    children: [],
  };
}

function walk(
  element: Element,
  document: Document,
  depth: number,
  budget: Budget,
): SemanticNode[] {
  if (SKIP_TAGS.has(element.tagName) || !isVisible(element)) return [];
  const interesting = isInteresting(element) || depth === 0;
  if (!interesting) {
    const promoted: SemanticNode[] = [];
    for (const child of Array.from(element.children)) {
      promoted.push(...walk(child, document, depth, budget));
      if (budget.truncated) break;
    }
    return promoted;
  }
  if (budget.nodes >= MAX_NODES) {
    budget.truncated = true;
    return [truncatedNode()];
  }
  budget.nodes += 1;
  const node: SemanticNode = {
    elementRef: stamp(element, document),
    role: roleOf(element) ?? element.tagName.toLowerCase(),
    states: statesOf(element, document),
    children: [],
  };
  const name = accessibleName(element);
  if (name) node.name = name;
  const value = valueOf(element);
  if (value !== undefined && value.length > 0) node.value = value;
  if (depth >= MAX_DEPTH) {
    if (element.children.length > 0) {
      budget.truncated = true;
      node.children.push(truncatedNode());
    }
    return [node];
  }
  for (const child of Array.from(element.children)) {
    if (budget.nodes >= MAX_NODES) {
      budget.truncated = true;
      node.children.push(truncatedNode());
      break;
    }
    node.children.push(...walk(child, document, depth + 1, budget));
  }
  return [node];
}

function snapshot(world: ShimWorld): ShimResult<{ page: PageState; root: SemanticNode }> {
  const document = world.document;
  const rootElement = document.body ?? document.documentElement;
  const budget: Budget = { nodes: 0, truncated: false };
  const [root] = walk(rootElement, document, 0, budget);
  return {
    ok: true,
    page: { url: world.location.href, title: document.title },
    root: root ?? truncatedNode(),
  };
}

function dispatchMouse(world: ShimWorld, element: Element, type: string): void {
  element.dispatchEvent(
    new world.MouseEvent(type, {
      bubbles: true,
      cancelable: true,
      composed: true,
      button: 0,
    }),
  );
}

function dispatchKey(
  world: ShimWorld,
  element: Element,
  type: string,
  key: string,
  modifiers: string[],
): boolean {
  const event = new world.KeyboardEvent(type, {
    bubbles: true,
    cancelable: true,
    composed: true,
    key,
    altKey: modifiers.includes("alt"),
    ctrlKey: modifiers.includes("control"),
    metaKey: modifiers.includes("meta"),
    shiftKey: modifiers.includes("shift"),
  });
  return element.dispatchEvent(event);
}

function dispatchInput(world: ShimWorld, element: Element, data: string): void {
  const InputEvent = world.InputEvent;
  if (InputEvent) {
    element.dispatchEvent(
      new InputEvent("input", {
        bubbles: true,
        cancelable: true,
        data,
        inputType: "insertText",
      }),
    );
    return;
  }
  element.dispatchEvent(new world.Event("input", { bubbles: true }));
}

function focus(element: Element): void {
  const focusFn = prop<(this: Element) => void>(element, "focus");
  focusFn?.call(element);
}

function click(world: ShimWorld, element: string): ShimResult<Record<string, never>> {
  const node = resolveRef(world.document, element);
  if (!node) return unresolved(element);
  focus(node);
  dispatchMouse(world, node, "pointerdown");
  dispatchMouse(world, node, "mousedown");
  dispatchMouse(world, node, "pointerup");
  dispatchMouse(world, node, "mouseup");
  // Native click() fires the click event and runs default actions
  // (submit, checkbox toggle, link navigation) that a synthetic MouseEvent
  // does not. isTrusted stays false.
  const nativeClick = prop<(this: Element) => void>(node, "click");
  if (nativeClick) nativeClick.call(node);
  else dispatchMouse(world, node, "click");
  return ok();
}

function insertText(element: Element, text: string): void {
  if (element.tagName === "INPUT" || element.tagName === "TEXTAREA") {
    const current = prop<string>(element, "value") ?? "";
    const start = prop<number | null>(element, "selectionStart") ?? current.length;
    const end = prop<number | null>(element, "selectionEnd") ?? start;
    const next = `${current.slice(0, start)}${text}${current.slice(end)}`;
    (element as FormControl).value = next;
    const cursor = start + text.length;
    prop<(this: Element, start: number, end: number) => void>(
      element,
      "setSelectionRange",
    )?.call(element, cursor, cursor);
    return;
  }
  if (element.hasAttribute("contenteditable") || prop<boolean>(element, "isContentEditable")) {
    element.textContent = `${element.textContent ?? ""}${text}`;
  }
}

function typeInto(
  world: ShimWorld,
  element: string,
  text: string,
): ShimResult<Record<string, never>> {
  const node = resolveRef(world.document, element);
  if (!node) return unresolved(element);
  focus(node);
  for (const char of text) {
    dispatchKey(world, node, "keydown", char, []);
    insertText(node, char);
    dispatchInput(world, node, char);
    dispatchKey(world, node, "keyup", char, []);
  }
  node.dispatchEvent(new world.Event("change", { bubbles: true }));
  return ok();
}

function press(
  world: ShimWorld,
  key: string,
  modifiers: string[] = [],
  element?: string | null,
): ShimResult<Record<string, never>> {
  let node: Element | null;
  if (element) {
    node = resolveRef(world.document, element);
    if (!node) return unresolved(element);
  } else {
    node = world.document.activeElement ?? world.document.body ?? world.document.documentElement;
  }
  focus(node);
  dispatchKey(world, node, "keydown", key, modifiers);
  dispatchKey(world, node, "keyup", key, modifiers);
  return ok();
}

function scroll(
  world: ShimWorld,
  deltaX: number,
  deltaY: number,
  element?: string | null,
): ShimResult<Record<string, never>> {
  if (element) {
    const node = resolveRef(world.document, element);
    if (!node) return unresolved(element);
    node.scrollTop += deltaY;
    node.scrollLeft += deltaX;
    node.dispatchEvent(new world.Event("scroll", { bubbles: true }));
    return ok();
  }
  const scrolling = world.document.scrollingElement ?? world.document.documentElement;
  scrolling.scrollTop += deltaY;
  scrolling.scrollLeft += deltaX;
  world.document.dispatchEvent(new world.Event("scroll", { bubbles: true }));
  return ok();
}

function drag(
  world: ShimWorld,
  sourceRef: string,
  targetRef: string,
): ShimResult<Record<string, never>> {
  const source = resolveRef(world.document, sourceRef);
  if (!source) return unresolved(sourceRef);
  const target = resolveRef(world.document, targetRef);
  if (!target) return unresolved(targetRef);
  const transfer: DataTransfer =
    typeof DataTransfer === "function"
      ? new DataTransfer()
      : ({ setData() {}, getData() { return ""; } } as unknown as DataTransfer);
  const dragEvent = (type: string, current: Element) => {
    const event = new world.MouseEvent(type, {
      bubbles: true,
      cancelable: true,
      composed: true,
    });
    Object.defineProperty(event, "dataTransfer", { value: transfer });
    current.dispatchEvent(event);
  };
  dragEvent("dragstart", source);
  dragEvent("dragenter", target);
  dragEvent("dragover", target);
  dragEvent("drop", target);
  dragEvent("dragend", source);
  return ok();
}

function waitFor(world: ShimWorld, predicate: WaitPredicate): ShimResult<{ holds: boolean }> {
  switch (predicate.predicate) {
    case "refResolve":
      return { ok: true, holds: resolveRef(world.document, predicate.element) !== null };
    case "refAbsent":
      return { ok: true, holds: resolveRef(world.document, predicate.element) === null };
    case "pageUrlContains":
      return { ok: true, holds: world.location.href.includes(predicate.needle) };
    case "pageTitleContains":
      return { ok: true, holds: world.document.title.includes(predicate.needle) };
    default:
      return {
        ok: false,
        error: {
          error: "unsupported",
          message: "wait_for admits only the four DOM-relative predicates",
        },
      };
  }
}

function installEventRing(world: ShimWorld): AgentControlApi["readEvents"] {
  const existing = world[SHIM_GLOBAL] as AgentControlApi | undefined;
  if (existing) return existing.readEvents.bind(existing);

  const ring: PageEvent[] = [];
  let nextSeq = 1;
  let dropped = 0;

  const push = (event: PageEvent) => {
    const recorded = event;
    nextSeq += 1;
    if (ring.length >= EVENT_BUFFER_LIMIT) {
      ring.shift();
      dropped += 1;
    }
    ring.push(recorded);
  };

  const wrap = (level: "log" | "info" | "warn" | "error" | "debug") => {
    const original = world.console[level]?.bind(world.console);
    world.console[level] = (...args: unknown[]) => {
      push({
        seq: nextSeq,
        kind: "console",
        level,
        text: args.map((value) => String(value)).join(" "),
      });
      original?.(...args);
    };
  };
  wrap("log");
  wrap("info");
  wrap("warn");
  wrap("error");
  wrap("debug");

  world.addEventListener("error", (event) => {
    const message =
      "message" in event && typeof event.message === "string"
        ? event.message
        : String(event);
    push({ seq: nextSeq, kind: "error", message });
  });
  world.addEventListener("unhandledrejection", (event) => {
    const reason =
      "reason" in event ? String((event as PromiseRejectionEvent).reason) : "unhandledrejection";
    push({ seq: nextSeq, kind: "error", message: reason });
  });

  const notifyNavigation = () => {
    push({ seq: nextSeq, kind: "navigation", url: world.location.href });
  };
  world.addEventListener("hashchange", notifyNavigation);
  world.addEventListener("popstate", notifyNavigation);
  const history = world.history;
  const pushState = history.pushState.bind(history);
  const replaceState = history.replaceState.bind(history);
  history.pushState = ((...args: Parameters<History["pushState"]>) => {
    pushState(...args);
    notifyNavigation();
  }) as History["pushState"];
  history.replaceState = ((...args: Parameters<History["replaceState"]>) => {
    replaceState(...args);
    notifyNavigation();
  }) as History["replaceState"];

  return (sinceSeq = 0) => ({
    events: ring.filter((event) => event.seq > sinceSeq),
    nextSeq,
    dropped,
  });
}

export function installAgentControlShim(world: ShimWorld): AgentControlApi {
  const existing = world[SHIM_GLOBAL] as AgentControlApi | undefined;
  if (existing) return existing;
  const readEvents = installEventRing(world);
  const api: AgentControlApi = {
    snapshot: () => snapshot(world),
    click: (element) => click(world, element),
    type: (element, text) => typeInto(world, element, text),
    press: (key, modifiers, element) => press(world, key, modifiers, element),
    scroll: (deltaX, deltaY, element) => scroll(world, deltaX, deltaY, element),
    drag: (source, target) => drag(world, source, target),
    waitFor: (predicate) => waitFor(world, predicate),
    readEvents,
  };
  world[SHIM_GLOBAL] = api;
  return api;
}
