# g01.004 Tauri Window Host And Lifecycle

Status: blocked on `g01.003`  
Owner: Tom  
Updated: 2026-07-27

## Outcome

Apply pure window plans to Tauri webview windows and capture durable geometry
without feedback loops.

## Batches

### 1. Host adapter

- monitor and live-window probes
- create, move, resize, show, focus, close, and reconcile
- capabilities and dynamic-window examples

### 2. Event lifecycle

- user versus programmatic move attribution
- debounced capture, scale-factor changes, close/shutdown flush
- primary coordinator identity and visible-on-ready flow

### 3. Proof

- mock-runtime command tests
- packaged restore on changed display arrangements
- Nucleus single-window and Loophole multi-window fixtures

## Acceptance

- restore cannot strand a window off-screen
- apply events do not recursively persist stale geometry
- clean close flushes; failed flush is observable
- the host works without layout or Surface packages

