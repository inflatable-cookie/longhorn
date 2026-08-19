(() => {
  // packages/longhorn/src/agent-control/shim.ts
  //! In-page semantic surface for contract 022: tree, live-DOM refs, synthetic
  //! input, wait_for predicates, and a bounded page-event ring.
  //!
  //! Pure page mechanics. No transport, no timers, no animation-frame waits.
  //! Refs live as attributes on the elements they name — there is no shim-side
  //! table that outlives the DOM. Truncation is a sentinel child with role
  //! `truncated` (the core vocabulary has no truncation field; a silent cap
  //! is forbidden). Synthetic events are untrusted: `isTrusted` stays false.
  var REF_ATTR = "data-longhorn-agent-ref";
  var REF_SEQ_ATTR = "data-longhorn-agent-ref-seq";
  var TRUNCATED_ROLE = "truncated";
  var TRUNCATED_REF = "truncated";
  var MAX_DEPTH = 24;
  var MAX_NODES = 200;
  var EVENT_BUFFER_LIMIT = 64;
  var SHIM_GLOBAL = "__longhornAgentControl";
  var SKIP_TAGS = new Set([
    "SCRIPT",
    "STYLE",
    "NOSCRIPT",
    "TEMPLATE",
    "HEAD",
    "META",
    "LINK",
    "TITLE",
    "IFRAME"
  ]);
  function prop(element, name) {
    return element[name];
  }
  function unresolved(element) {
    return { ok: false, error: { error: "unresolvedRef", element } };
  }
  function ok() {
    return { ok: true };
  }
  function resolveRef(document, element) {
    if (element === TRUNCATED_REF || element.length === 0)
      return null;
    return document.querySelector(`[${REF_ATTR}="${element}"]`);
  }
  function allocRef(document) {
    const root = document.documentElement;
    const current = Number(root.getAttribute(REF_SEQ_ATTR) ?? "0");
    const next = Number.isFinite(current) ? current + 1 : 1;
    root.setAttribute(REF_SEQ_ATTR, String(next));
    return `e${next}`;
  }
  function stamp(element, document) {
    const existing = element.getAttribute(REF_ATTR);
    if (existing)
      return existing;
    const id = allocRef(document);
    element.setAttribute(REF_ATTR, id);
    return id;
  }
  function computedStyle(element) {
    const view = element.ownerDocument.defaultView;
    return view ? view.getComputedStyle(element) : null;
  }
  function isVisible(element) {
    if (element.hasAttribute("hidden"))
      return false;
    if (element.getAttribute("aria-hidden") === "true")
      return false;
    const style = computedStyle(element);
    if (!style)
      return true;
    return style.display !== "none" && style.visibility !== "hidden";
  }
  function implicitRole(element) {
    const tag = element.tagName;
    if (tag === "A")
      return element.hasAttribute("href") ? "link" : null;
    if (tag === "BUTTON")
      return "button";
    if (tag === "TEXTAREA")
      return "textbox";
    if (tag === "SELECT")
      return "combobox";
    if (tag === "OPTION")
      return "option";
    if (tag === "SUMMARY")
      return "button";
    if (tag === "DETAILS")
      return "group";
    if (tag === "NAV")
      return "navigation";
    if (tag === "MAIN")
      return "main";
    if (tag === "HEADER")
      return "banner";
    if (tag === "FOOTER")
      return "contentinfo";
    if (tag === "FORM")
      return "form";
    if (tag === "UL" || tag === "OL")
      return "list";
    if (tag === "LI")
      return "listitem";
    if (tag === "TABLE")
      return "table";
    if (tag === "DIALOG")
      return "dialog";
    if (tag === "IMG")
      return "image";
    if (tag === "H1" || tag === "H2" || tag === "H3" || tag === "H4" || tag === "H5" || tag === "H6") {
      return "heading";
    }
    if (tag === "INPUT") {
      const type = (element.getAttribute("type") ?? "text").toLowerCase();
      if (type === "checkbox")
        return "checkbox";
      if (type === "radio")
        return "radio";
      if (type === "number")
        return "spinbutton";
      if (type === "range")
        return "slider";
      if (type === "button" || type === "submit" || type === "reset" || type === "file") {
        return "button";
      }
      if (type === "hidden")
        return null;
      return "textbox";
    }
    if (element.hasAttribute("contenteditable"))
      return "textbox";
    return null;
  }
  function roleOf(element) {
    const explicit = element.getAttribute("role");
    if (explicit === "none" || explicit === "presentation")
      return null;
    if (explicit)
      return explicit;
    return implicitRole(element);
  }
  function labelledByText(element) {
    const ids = element.getAttribute("aria-labelledby");
    if (!ids)
      return;
    const document = element.ownerDocument;
    const text = ids.split(/\s+/).map((id) => document.getElementById(id)?.textContent ?? "").join(" ").replace(/\s+/g, " ").trim();
    return text.length > 0 ? text : undefined;
  }
  function associatedLabel(element) {
    const id = element.id;
    const document = element.ownerDocument;
    if (id) {
      const label = document.querySelector(`label[for="${id}"]`);
      const text = label?.textContent?.replace(/\s+/g, " ").trim();
      if (text)
        return text;
    }
    const parent = element.closest("label");
    if (parent) {
      const text = parent.textContent?.replace(/\s+/g, " ").trim();
      if (text)
        return text;
    }
    return;
  }
  function accessibleName(element) {
    const labelled = labelledByText(element);
    if (labelled)
      return labelled;
    const aria = element.getAttribute("aria-label")?.trim();
    if (aria)
      return aria;
    const label = associatedLabel(element);
    if (label)
      return label;
    if (element.tagName === "IMG") {
      const alt = element.getAttribute("alt")?.trim();
      if (alt)
        return alt;
    }
    const role = roleOf(element);
    if (role === "button" || role === "link" || role === "heading" || role === "option" || role === "tab" || role === "listitem") {
      const text = element.textContent?.replace(/\s+/g, " ").trim();
      if (text)
        return text;
    }
    const title = element.getAttribute("title")?.trim();
    if (title)
      return title;
    const placeholder = element.getAttribute("placeholder")?.trim();
    if (placeholder)
      return placeholder;
    return;
  }
  function valueOf(element) {
    const now = element.getAttribute("aria-valuenow");
    if (now)
      return now;
    if (element.tagName === "INPUT") {
      const type = (prop(element, "type") ?? "text").toLowerCase();
      if (type === "checkbox" || type === "radio")
        return;
      return prop(element, "value");
    }
    if (element.tagName === "TEXTAREA")
      return prop(element, "value");
    if (element.tagName === "SELECT") {
      const selected = prop(element, "selectedOptions");
      const first = selected && selected.length > 0 ? selected[0] : undefined;
      return first?.textContent?.trim() ?? prop(element, "value");
    }
    if (element.hasAttribute("contenteditable")) {
      return element.textContent ?? undefined;
    }
    return;
  }
  function statesOf(element, document) {
    const states = [];
    if (isVisible(element))
      states.push("visible");
    if (element.hasAttribute("disabled") || element.getAttribute("aria-disabled") === "true" || prop(element, "disabled") === true) {
      states.push("disabled");
    }
    if (prop(element, "checked") === true || element.getAttribute("aria-checked") === "true") {
      states.push("checked");
    }
    if (document.activeElement === element)
      states.push("focused");
    if (element.getAttribute("aria-expanded") === "true")
      states.push("expanded");
    if (element.getAttribute("aria-pressed") === "true")
      states.push("pressed");
    if (element.getAttribute("aria-selected") === "true" || prop(element, "selected") === true) {
      states.push("selected");
    }
    if (element.getAttribute("aria-readonly") === "true" || prop(element, "readOnly") === true) {
      states.push("readonly");
    }
    return states;
  }
  function isInteresting(element) {
    if (SKIP_TAGS.has(element.tagName))
      return false;
    if (element.tagName === "LABEL")
      return false;
    if (roleOf(element))
      return true;
    if (accessibleName(element))
      return true;
    if (element.tagName === "INPUT")
      return true;
    if (element.hasAttribute("tabindex"))
      return true;
    return false;
  }
  function truncatedNode() {
    return {
      elementRef: TRUNCATED_REF,
      role: TRUNCATED_ROLE,
      states: [],
      children: []
    };
  }
  function walk(element, document, depth, budget) {
    if (SKIP_TAGS.has(element.tagName) || !isVisible(element))
      return [];
    const interesting = isInteresting(element) || depth === 0;
    if (!interesting) {
      const promoted = [];
      for (const child of Array.from(element.children)) {
        promoted.push(...walk(child, document, depth, budget));
        if (budget.truncated)
          break;
      }
      return promoted;
    }
    if (budget.nodes >= MAX_NODES) {
      budget.truncated = true;
      return [truncatedNode()];
    }
    budget.nodes += 1;
    const node = {
      elementRef: stamp(element, document),
      role: roleOf(element) ?? element.tagName.toLowerCase(),
      states: statesOf(element, document),
      children: []
    };
    const name = accessibleName(element);
    if (name)
      node.name = name;
    const value = valueOf(element);
    if (value !== undefined && value.length > 0)
      node.value = value;
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
  function snapshot(world) {
    const document = world.document;
    const rootElement = document.body ?? document.documentElement;
    const budget = { nodes: 0, truncated: false };
    const [root] = walk(rootElement, document, 0, budget);
    return {
      ok: true,
      page: { url: world.location.href, title: document.title },
      root: root ?? truncatedNode()
    };
  }
  function dispatchMouse(world, element, type) {
    element.dispatchEvent(new world.MouseEvent(type, {
      bubbles: true,
      cancelable: true,
      composed: true,
      button: 0
    }));
  }
  function dispatchKey(world, element, type, key, modifiers) {
    const event = new world.KeyboardEvent(type, {
      bubbles: true,
      cancelable: true,
      composed: true,
      key,
      altKey: modifiers.includes("alt"),
      ctrlKey: modifiers.includes("control"),
      metaKey: modifiers.includes("meta"),
      shiftKey: modifiers.includes("shift")
    });
    return element.dispatchEvent(event);
  }
  function dispatchInput(world, element, data) {
    const InputEvent = world.InputEvent;
    if (InputEvent) {
      element.dispatchEvent(new InputEvent("input", {
        bubbles: true,
        cancelable: true,
        data,
        inputType: "insertText"
      }));
      return;
    }
    element.dispatchEvent(new world.Event("input", { bubbles: true }));
  }
  function focus(element) {
    const focusFn = prop(element, "focus");
    focusFn?.call(element);
  }
  function click(world, element) {
    const node = resolveRef(world.document, element);
    if (!node)
      return unresolved(element);
    focus(node);
    dispatchMouse(world, node, "pointerdown");
    dispatchMouse(world, node, "mousedown");
    dispatchMouse(world, node, "pointerup");
    dispatchMouse(world, node, "mouseup");
    const nativeClick = prop(node, "click");
    if (nativeClick)
      nativeClick.call(node);
    else
      dispatchMouse(world, node, "click");
    return ok();
  }
  function insertText(element, text) {
    if (element.tagName === "INPUT" || element.tagName === "TEXTAREA") {
      const current = prop(element, "value") ?? "";
      const start = prop(element, "selectionStart") ?? current.length;
      const end = prop(element, "selectionEnd") ?? start;
      const next = `${current.slice(0, start)}${text}${current.slice(end)}`;
      element.value = next;
      const cursor = start + text.length;
      prop(element, "setSelectionRange")?.call(element, cursor, cursor);
      return;
    }
    if (element.hasAttribute("contenteditable") || prop(element, "isContentEditable")) {
      element.textContent = `${element.textContent ?? ""}${text}`;
    }
  }
  function typeInto(world, element, text) {
    const node = resolveRef(world.document, element);
    if (!node)
      return unresolved(element);
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
  function press(world, key, modifiers = [], element) {
    let node;
    if (element) {
      node = resolveRef(world.document, element);
      if (!node)
        return unresolved(element);
    } else {
      node = world.document.activeElement ?? world.document.body ?? world.document.documentElement;
    }
    focus(node);
    dispatchKey(world, node, "keydown", key, modifiers);
    dispatchKey(world, node, "keyup", key, modifiers);
    return ok();
  }
  function scroll(world, deltaX, deltaY, element) {
    if (element) {
      const node = resolveRef(world.document, element);
      if (!node)
        return unresolved(element);
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
  function drag(world, sourceRef, targetRef) {
    const source = resolveRef(world.document, sourceRef);
    if (!source)
      return unresolved(sourceRef);
    const target = resolveRef(world.document, targetRef);
    if (!target)
      return unresolved(targetRef);
    const transfer = typeof DataTransfer === "function" ? new DataTransfer : { setData() {}, getData() {
      return "";
    } };
    const dragEvent = (type, current) => {
      const event = new world.MouseEvent(type, {
        bubbles: true,
        cancelable: true,
        composed: true
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
  function waitFor(world, predicate) {
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
            message: "wait_for admits only the four DOM-relative predicates"
          }
        };
    }
  }
  function installEventRing(world) {
    const existing = world[SHIM_GLOBAL];
    if (existing)
      return existing.readEvents.bind(existing);
    const ring = [];
    let nextSeq = 1;
    let dropped = 0;
    const push = (event) => {
      const recorded = event;
      nextSeq += 1;
      if (ring.length >= EVENT_BUFFER_LIMIT) {
        ring.shift();
        dropped += 1;
      }
      ring.push(recorded);
    };
    const wrap = (level) => {
      const original = world.console[level]?.bind(world.console);
      world.console[level] = (...args) => {
        push({
          seq: nextSeq,
          kind: "console",
          level,
          text: args.map((value) => String(value)).join(" ")
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
      const message = "message" in event && typeof event.message === "string" ? event.message : String(event);
      push({ seq: nextSeq, kind: "error", message });
    });
    world.addEventListener("unhandledrejection", (event) => {
      const reason = "reason" in event ? String(event.reason) : "unhandledrejection";
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
    history.pushState = (...args) => {
      pushState(...args);
      notifyNavigation();
    };
    history.replaceState = (...args) => {
      replaceState(...args);
      notifyNavigation();
    };
    return (sinceSeq = 0) => ({
      events: ring.filter((event) => event.seq > sinceSeq),
      nextSeq,
      dropped
    });
  }
  function installAgentControlShim(world) {
    const existing = world[SHIM_GLOBAL];
    if (existing)
      return existing;
    const readEvents = installEventRing(world);
    const api = {
      snapshot: () => snapshot(world),
      click: (element) => click(world, element),
      type: (element, text) => typeInto(world, element, text),
      press: (key, modifiers, element) => press(world, key, modifiers, element),
      scroll: (deltaX, deltaY, element) => scroll(world, deltaX, deltaY, element),
      drag: (source, target) => drag(world, source, target),
      waitFor: (predicate) => waitFor(world, predicate),
      readEvents
    };
    world[SHIM_GLOBAL] = api;
    return api;
  }

  // packages/longhorn/src/agent-control/inject.ts
  var target = globalThis;
  installAgentControlShim(target);
})();
