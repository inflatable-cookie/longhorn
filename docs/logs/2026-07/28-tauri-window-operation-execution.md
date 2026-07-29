# Tauri Window Operation Execution

Date: 2026-07-28
State: complete implementation batch

## Outcome

- completed Card 018
- added a strict managed handle-to-`WindowId` registry
- protected the explicit primary slot independently of planner policy
- added caller-owned dynamic `WebviewWindow` creation
- derived host capabilities from native support and factory availability
- executed every Card 016 operation through distinct native calls
- added asynchronous Tauri main-thread dispatch
- added typed success, partial-failure, and dependency-skip attempts
- registered full expected-operation evidence before native mutation
- added complete fresh readback and convergence planning
- made Card 019 the sole ready lane

## Identity And Creation

Tauri labels remain opaque `HostWindowHandle` values. Initial registry state
rejects duplicate handles, duplicate stable ids, and an absent protected
primary. Retag changes registry bookkeeping only.

Dynamic creation is an injected consumer seam. URL, title, chrome, minimum
size, capabilities, and product metadata stay outside Longhorn. The factory
must return a hidden unmaximized window. Longhorn validates that state and
manages the returned handle only after validation and unique identity checks
succeed.

## Apply Policy

The adapter replaces caller capability claims with capabilities it can execute.
Create is advertised only when the injected factory is available.

Execution preserves Card 016 order and is nontransactional. Outer position and
inner size are separate native calls, so a resize failure after a successful
move remains visible. Failure blocks later operations for the same stable
window. Independent windows continue.

The registry installs the apply generation and full expected
`WindowOperation` before each native call. Close evidence is included. A stale
generation fails before execution. Protected-primary close is refused even if
the pure planner was given no protected-slot policy.

## Readback Policy

Every apply requests a new complete Card 017 desktop observation. The original
desired state is diffed against those fresh live windows. Intended calls never
edit or substitute for readback. Observation failure remains in the receipt;
successful calls do not claim convergence without evidence.

Close success does not remove registry bookkeeping. Native close is a request,
and only later observation or lifecycle events may prove destruction.

## Evidence

- protected-primary retag keeps the native label and installs stable identity
- injected hidden creation succeeds; factory failure and visible results stay
  unmanaged
- unmaximize, outer move, inner resize, maximize, show, hide, focus, and close
  map to distinct host-call fixtures
- successful move followed by failed resize reports the completed and failed
  calls separately
- later same-window operations skip after failure while another window hides
- stale generation is rejected before native calls
- programmatic close evidence retains generation, stable id, handle, and full
  operation
- mismatching readback reports remaining work
- matching readback and repeated matching apply are empty
- full Rust 1.85 Effigy QA passed before a module-only split made to clear the
  hard god-file threshold
- after that split, formatting, warnings-denied Clippy, test compilation,
  Effigy docs, Northstar, and the god-file scan pass

The first post-split executable rerun stalled under macOS `syspolicyd` before
the Rust harness. A later full Effigy test run executed all workspace and
documentation tests successfully. The temporary host validation exception is
closed.

## Boundary

No event listener, persistence, retry, rollback, layout, Surface, TypeScript,
Svelte, Poodle, product window definition, or donor write entered the package.

## Posture

`strict-ready`

Card 018 is complete. Card 019 is ready after reassessment against exact
expected-operation evidence, attempt receipts, and convergence readback.

## Next

Review and explicitly start Card 019. Do not start event attribution
automatically.
