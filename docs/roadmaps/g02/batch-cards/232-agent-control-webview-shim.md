# 232 Agent Control Webview Shim

Status: ready
Owner: Longhorn maintainers
Roadmap: g02.032
Governing refs: contract 022; contracts 010, 013; memo 024
Depends on: Cards 230-231 (merged, PR 4)
Auto-start next card: yes — Card 233 in the same lane

## Objective

The in-page half of the semantic surface exists in the `longhorn` TS
package: a shim the plugin injects in dev that builds the semantic tree,
stamps and resolves refs, dispatches synthetic input, and evaluates
`wait_for` predicates — pure page-side mechanics, no transport.

## Scope

- **Semantic tree.** Walk the DOM into the core vocabulary's
  `SemanticNode` shape: role (ARIA-first, tag-derived fallback),
  accessible name, value, and interaction state (disabled, checked,
  focused, visible). Bounded depth/size with an explicit truncation
  marker — a silent cap reads as "covered everything" and must not exist.
- **Refs.** Stamp stable ids onto elements at snapshot time
  (`data-longhorn-agent-ref` or equivalent) and resolve them against the
  live DOM on use, per contract 022: no shim-side table that outlives the
  DOM. A ref whose element is gone resolves to the vocabulary's
  `UnresolvedRef`, never to a guess.
- **Synthetic input.** `click`, `type` (per-key events plus value commit
  and `input`/`change`), `press` with modifiers, `scroll`, and `drag` as
  the documented untrusted DOM sequence (dragstart → dragover → drop).
  Nothing touches OS input; `isTrusted` stays false and that limit is
  documented at the API, per contract 022.
- **`wait_for` evaluation.** Evaluate the four DOM-relative predicates
  against the live DOM, poll driven by the Rust side (the shim answers
  "holds now?"; pacing and timeout stay host-side). No timer or rAF
  waiting in the shim — contract 022's DOM-relative rule.
- **Delivery.** Shipped from the `longhorn` package as a self-contained
  injectable script the plugin can load (contract 013 packaging rules);
  no runtime dependency on app code, no effect when the control server is
  absent.

Testing: DOM-level fixtures (vitest + jsdom or the package's existing
harness) for tree shape, ref staleness, input event sequences, truncation
marking, and each predicate. Browser-real behavior is Card 234's packaged
proof; keep these fixtures about logic, not pixels.

## Acceptance Criteria

- [ ] semantic tree renders roles, names, values, state, and an explicit
      truncation marker per the core vocabulary
- [ ] refs resolve against the live DOM; a removed element yields
      `UnresolvedRef`; two snapshots interleaved from two clients do not
      invalidate each other's refs
- [ ] input dispatch emits the documented event sequences on real
      handlers, `type` reaching both key handlers and value listeners
- [ ] no shim path waits on time or animation frames
- [ ] shim bundles standalone from the `longhorn` package; TS package
      gates (contract 013 / `effigy qa`) pass
- [ ] `effigy qa` passes

## Validation

`effigy qa`; the package's TS test suite.

## Stop Conditions

- the semantic tree needs a concept the core vocabulary cannot express —
  extend nothing silently; that is a vocabulary question for the
  orchestrator (core surface change = report first);
- ref stability cannot be kept without a shim-side registry that outlives
  the DOM — that contradicts contract 022's stateless posture.

## Continuation

Card 233 wires this shim to the plugin's tools in the same lane.
