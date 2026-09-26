# Compose Agent App Control

Status: checked private adoption guidance
Updated: 2026-09-26
Governing contracts: [022](../contracts/022-agent-app-control.md),
[003](../contracts/003-extraction-and-consumer-migration.md),
[006](../contracts/006-command-action-and-input.md),
[012](../contracts/012-distribution-and-compatibility.md)

## Why This Matters

Agents testing a Longhorn app through OS computer use steal focus and the
pointer. The contract 022 control surface is the replacement: a
compile-time opt-in MCP server inside the app. This guide is the Rust half
— how a consumer app mounts it. The agent half is the skill at
`skills/agent-control/`; install it with
`effigy agent-control:install-skill -- <git-repo>` (from the Longhorn
checkout).

A consumer card should execute from this page plus the worked example. Do
not copy product policy out of another app.

## Worked Example

[`examples/agent-control-proof`](../../examples/agent-control-proof) is the
complete composition: `mount_agent_control` from `setup`, a `CommandBridge`
over a sealed contract-006 registry, both exit hooks, and a form-and-list
UI an agent can drive unfocused. It is never shipped. Copy structure from
it; replace identity, commands, and UI.

Packaged launch (macOS, operator's display):

```sh
cd examples/agent-control-proof
bunx @tauri-apps/cli build
open -g -a ../../target/release/bundle/macos/Longhorn\ Agent\ Control\ Proof.app
```

`open -g` launches without stealing focus. Discovery appears at
`~/Library/Application Support/longhorn/state/agent-control/dev.example.longhorn-agent-control-proof-<pid>.json`.

## 1. The `agent-control` Feature Is A Compile-Time Opt-In

The entire surface sits behind `longhorn-tauri-agent-control`'s
off-by-default `agent-control` cargo feature. A featureless build is an
empty library: no server, route, token, discovery, or shim code, and no
runtime toggle can enable it. The application starts the server with
`mount_agent_control`; enabling the feature does not start it. Longhorn's
`effigy check:agent-control-release-absence` proves three states for this
repo: neither feature (no surface), `agent-control` only (server present,
`evaluate` omitted), and both features (`evaluate` present). A consumer
must keep the same compile-time gate.

```toml
# src-tauri/Cargo.toml
[dependencies]
longhorn-tauri-agent-control = "0.1.0"

[features]
agent-control = ["longhorn-tauri-agent-control/agent-control"]
# Dev/test only. Packaged builds omit this; `evaluate` then answers typed
# Unsupported. The registered command catalogue is the allowed agency.
agent-control-evaluate = [
  "agent-control",
  "longhorn-tauri-agent-control/agent-control-evaluate",
]
```

A packaged consumer enables `agent-control`, not `agent-control-evaluate`.
Loopback binding, the per-instance bearer token, and Origin validation do
not change. The proof app enables both features unconditionally because
that app is not a product.

The symbols (`mount_agent_control`, `CommandBridge`, `AgentControlConfig`)
exist only with `agent-control`. Gate the composition:

```rust
#[cfg(feature = "agent-control")]
{
    // mount here
}
```

## 2. Implement `CommandBridge`

The plugin holds no command authority. `command` travels the same
contract-006 path a menu or palette would. The app supplies a
`CommandBridge` over its own sealed registry and admission engine.

```rust
use longhorn_core::CommandId;
use longhorn_tauri_agent_control::{CommandBridge, ToolError};
use serde_json::Value;

struct AppCommandBridge { /* registry, executor, app handle */ }

impl CommandBridge for AppCommandBridge {
    fn invoke_command(
        &self,
        command: &CommandId,
        argument: Option<Value>,
    ) -> Result<Option<Value>, ToolError> {
        // Admit and execute through the app's registry.
        // Map every failure to ToolError::CommandFailed — never panic.
        todo!()
    }
}
```

The bridge runs on the control-server thread: keep it non-blocking with
respect to the surface. Native menus and dialogs are out of scope for
click/type — expose that behavior as registered commands and let the agent
call `command`. The proof registry (`proof:ping`,
`proof:window.minimize`, `proof:window.restore`) is the pattern, not the
catalogue to ship.

### Windows hosting child webviews

Apps that attach child webviews (native-content islands, preview panes)
to a window stay targetable: the handler enumerates `Window`s and, by
default, drives the webview sharing the window's label. Child webviews
are screenshot surfaces always (Card 238) and semantic targets only when
the app names their labels at mount:

```rust
AgentControlConfig::new(APP_ID).with_semantic_child("preview")
```

Repeat `with_semantic_child` for each label. The set is fixed at mount;
there is no runtime mutation. Opting in a label asserts that child's
content is the app's own to drive — `evaluate` and synthetic input
execute inside whatever that webview hosts. Do **not** opt in labels
that host third-party content (`longhorn-browser` views are the named
counterexample). Default is closed: an unnamed child answers typed
`Unsupported` naming the opt-in absence; a label that matches no hosted
webview answers `UnknownWebview`.

Agents address an opted-in child with the optional `webview` argument on
`snapshot`, `click`, `type`, `press`, `scroll`, `drag`, `wait_for`, and
`evaluate`. Omit it and the call is the UI webview — today's wire,
unchanged. Refs are scoped to the webview that stamped them; crossing
them is `UnresolvedRef`, never a wrong-element hit.

Untrusted `drag` is ref-to-ref and two-point (source center → target
center): it dispatches pointer/mouse down-move-up plus the HTML5 DnD
sequence. It does not interpolate a pixel path, so a free-form marquee
that only samples intermediate mousemove coordinates is not expressed.

`screenshot` still composes the whole window from every hosted webview's
own snapshot, each drawn at its physical bounds in view-hierarchy z-order
and clipped to the window (Card 238). A hidden child contributes nothing,
matching the real window; a child whose snapshot fails fails the call
typed rather than silently dropping out of the image. Freshness holds per
webview in every probed window state (frontmost, unfocused, occluded,
minimized). A genuinely native (non-webview) island is not captured — no
provider ships for that seam. The plugin's `agent-control` feature enables
tauri's `unstable` feature for this; builds without `agent-control` are
unaffected because the whole dependency is feature-gated.

### Applications without a command registry

Leaving contract 006 uncomposed is a supported composition, not a gap. Do
**not** adopt `longhorn-command` just to satisfy this guide, and do not
bridge Tauri `invoke` handlers into `command` — that would hand the agent
a route your registry and admission engine never authorized (contract 022
adds no authority). Mount with the provided no-command bridge instead:

```rust
use longhorn_tauri_agent_control::NoCommandBridge;

mount_agent_control(app.handle(), config, std::sync::Arc::new(NoCommandBridge))?;
```

Every `command` call then answers a typed `Unsupported` naming the
absence. The consequence to accept knowingly: behavior reachable only
through native menus or dialogs has **no** agent path in such an app —
agents drive whatever the UI exposes to the semantic tools, and the skill
tells them to report the gap rather than click native chrome.

## 3. Mount From `setup`

```rust
#[cfg(feature = "agent-control")]
use longhorn_tauri_agent_control::{
    AgentControlConfig, AgentControlHandle, mount_agent_control,
};

const APP_ID: &str = "com.example.app"; // canonical application id

#[cfg(feature = "agent-control")]
struct AgentControlState {
    agent_control: std::sync::Mutex<Option<AgentControlHandle>>,
}

// inside tauri::Builder::setup:
#[cfg(feature = "agent-control")]
{
    let bridge = std::sync::Arc::new(AppCommandBridge::new(/* ... */));
    let agent_control = mount_agent_control(
        app.handle(),
        AgentControlConfig::new(APP_ID)
            .with_semantic_child("preview"), // omit if no child needs driving
        bridge,
    )?;
    app.manage(AgentControlState {
        agent_control: std::sync::Mutex::new(Some(agent_control)),
    });
}
```

`AgentControlConfig::new` binds `127.0.0.1` on an ephemeral port and
publishes the discovery file after bind. `with_port` pins a port;
`with_state_root` is deployment/test policy (contract 004), not a second
discovery location agents have to guess. `with_semantic_child` is the
opt-in above; skip it when no child webview should be a semantic target.

Mount injects the in-page shim as an initialization script. The app does
not mount a separate JavaScript package for snapshot or input.

## 3b. Replace plugin-dialog at picker call sites

Do not import `@tauri-apps/plugin-dialog` from UI handlers that an agent
must drive. Bind Longhorn's replacement and pass the plugin functions
through — Longhorn does not depend on the plugin package:

```ts
import { bindFileSelection } from "@inflatable-cookie/longhorn/agent-control";
import { open as pluginOpen, save as pluginSave } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

export const { open, save } = bindFileSelection({
  invoke: (command, args) => invoke(command, args),
  pluginOpen,
  pluginSave,
});
```

Register the begin-selection command next to your other Longhorn handlers
and allow it on the window that hosts picker call sites:

```rust
.invoke_handler(tauri::generate_handler![
    longhorn_tauri_agent_control::longhorn_agent_control_begin_selection,
])
```

```toml
# src-tauri/permissions/agent-control.toml
[[permission]]
identifier = "allow-begin-selection"
description = "Allows the renderer to begin an agent-originated file or folder picker."
commands.allow = ["longhorn_agent_control_begin_selection"]
```

```json
"permissions": ["core:default", "allow-begin-selection"]
```

Agent-originated `open`/`save` (synthetic click/type/press/drag, or
`evaluate`) publish a pending request and wait for `answer_selection` /
`reject_selection`. Human-originated calls pass the original options to
plugin-dialog unchanged, even while the control server is running. An
active server is not evidence that a human picker should be captured.
Do not globally monkey-patch the plugin. `save` selects a target path;
Longhorn never writes it. HTML file inputs stay outside this seam.
Rust-side pickers join it when the consumer carries origin — see 3c.

The `agent-control` feature is required for the begin-selection command
and shim origin tracking. A default build has neither; keep calling
plugin-dialog there.

## 3c. Carry origin into a Rust picker command

JS `open`/`save` can read the page shim at the call site. A Rust command
that opens a native picker (`blocking_save_file`, `blocking_pick_file`,
and the rest) cannot. The page passes the origin token in that command's
arguments; Longhorn never infers origin on the host.

Read the token from the installed shim. No shim is human:

```ts
import { currentSelectionOrigin } from "@inflatable-cookie/longhorn/agent-control";
import { invoke } from "@tauri-apps/api/core";

await invoke("export_backup", { origin: currentSelectionOrigin() });
```

Before, the command owned the dialog plugin call:

```rust,ignore
#[tauri::command]
fn export_backup(app: tauri::AppHandle) -> Result<(), String> {
    let Some(path) = app.dialog().file().blocking_save_file() else {
        return Ok(());
    };
    write_backup(&path)
}
```

After, the same command keeps its dialog options and result handling.
The only addition is the origin token and one match on the host entry.
Take `origin: Option<SelectionOrigin>` and `unwrap_or_default()`: a
missing invoke key is human. `#[serde(default)]` on a command parameter
does not compile. A required `SelectionOrigin` argument rejects the
invoke (`missing required key origin`) instead of opening the plugin.

```rust,ignore
#[tauri::command]
async fn export_backup(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    registry: tauri::State<'_, longhorn_tauri_agent_control::SelectionRegistry>,
    origin: Option<longhorn_tauri_agent_control::SelectionOrigin>,
) -> Result<(), String> {
    let path = match longhorn_tauri_agent_control::begin_host_selection(
        origin.unwrap_or_default(),
        &webview,
        &registry,
        longhorn_tauri_agent_control::BeginSelectionArgs {
            kind: longhorn_tauri_agent_control::SelectionKind::Save,
            directory: false,
            multiple: false,
            filters: Vec::new(),
            title: Some("Export backup".into()),
            default_path: None,
        },
    )
    .await
    {
        longhorn_tauri_agent_control::HostSelection::UsePlugin => {
            let Some(path) = app.dialog().file().blocking_save_file() else {
                return Ok(());
            };
            path
        }
        longhorn_tauri_agent_control::HostSelection::Agent(result) => match result? {
            serde_json::Value::String(path) => std::path::PathBuf::from(path),
            serde_json::Value::Null => return Ok(()),
            other => return Err(format!("unexpected selection {other}")),
        },
    };
    write_backup(&path)
}
```

Human, missing, and malformed origin take the `UsePlugin` arm and never
publish a pending request. Agent origin waits for `answer_selection` /
`reject_selection` with the same result shape as the JS route: one path
or `null` for save. There is no native fallback on the agent arm, and
Longhorn never writes the chosen path.

## 4. Hook Both `ExitRequested` And `Exit`

Clean shutdown removes the discovery file (it carries the bearer token).
A crash leaves the file stale-detectable by dead pid. Dropping the handle
without `shutdown` strands the file.

macOS quit delivers `RunEvent::Exit` without a preceding `ExitRequested`.
Hooking only `ExitRequested` strands the discovery file on clean quit.
Hook both. `Option::take` makes the second fire a no-op.

```rust
app.run(|app, event| {
    #[cfg(feature = "agent-control")]
    if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = event
        && let Some(state) = app.try_state::<AgentControlState>()
        && let Some(agent_control) = state.agent_control.lock().expect("state poisoned").take()
    {
        let _ = agent_control.shutdown();
    }
});
```

## What The App Gets

Once mounted, an agent can:

| Tool | What it does |
| --- | --- |
| `snapshot` | semantic tree with live-DOM refs |
| `click`, `type`, `press`, `scroll`, `drag` | untrusted in-page DOM events; never moves the OS pointer; never requires focus |
| `evaluate` | JS in the page; escape hatch; full code execution; omitted without `agent-control-evaluate` (typed `Unsupported`) |
| `wait_for` | DOM-relative predicates only |
| `screenshot` | fresh image of the whole window, child webviews composed in; occluded, unfocused, and minimized; macOS only |
| `command` | invoke a registered contract-006 command by id; in a packaged build this catalogue is the allowed agency |
| `answer_selection`, `reject_selection` | settle an agent-originated JS or Rust picker; human pickers keep the dialog plugin |
| `list_windows`, `resize_window` | window scope |

Page events ride `subscriptions/listen` as `resources/updated` on
`longhorn://agent-control/{console,error,navigation,selection}`. Selection
carries pending picker state so a late subscriber can still answer.

Discovery lives under the contract 004 `longhorn` identity's state root
plus `agent-control/`, not under the app's own storage identity — so one
directory lists every live instance. File name is `<app-id>-<pid>.json`.

## What The App Must Not Expect

- Native menus, native dialogs, or OS-level input. Use `command`. File
  and folder pickers that go through `bindFileSelection` or
  `begin_host_selection` are answered over MCP; do not expect Longhorn to
  drive an already-open OS panel.
- Trusted events (`isTrusted`, native hover, OS drag-and-drop). Synthetic
  input is untrusted by contract.
- Capture, `evaluate`, or the semantic tools on non-macOS hosts. Those
  compile and answer typed `Unsupported` (contract 020).
- The server without `agent-control`. Absence is the feature. Enabling
  `agent-control` still does not start the server — the application calls
  `mount_agent_control`.
- `evaluate` in a packaged `agent-control` build. That tool answers typed
  `Unsupported` unless `agent-control-evaluate` is also enabled.
- Time-only or animation-frame waits. WKWebView coalesces timers in every
  window state and stops `requestAnimationFrame` while the window is not
  key. `wait_for` is DOM-relative on purpose.
- A second agent needing coordination. Instances are interleave-safe;
  refs are shared; pick by app id and pid.

## Install The Skill

From this Longhorn checkout, copy the canonical skill into a consumer git
repo. Prefer the Effigy task. Pass the consumer path after `--` (positional
or `--repo`); do not put `--repo` before the task name — that switches
catalogs and the consumer does not define the installer.

```sh
effigy agent-control:install-skill -- /path/to/consumer
effigy agent-control:install-skill -- --repo /path/to/consumer
```

The bun script remains the implementation if you need to invoke it
directly: `bun scripts/install-agent-control-skill.ts /path/to/consumer`.

The copy lands at `.claude/skills/agent-control/` in the target. Re-run
at the same version is a no-op. This is operator-invoked, never automatic
(contract 003).
