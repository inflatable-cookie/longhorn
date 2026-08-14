const { invoke } = window.__TAURI__.core;
const currentWindow = window.__TAURI__.window.getCurrentWindow();
const label = currentWindow.label;
const status = document.querySelector("#status");
const message = document.querySelector("#message");
const dragRegion = document.querySelector("header");

document.querySelector("#window-label").textContent = label;
document.querySelector("#main-controls").hidden = label !== "main";
document.querySelector("#workspace-controls").hidden = label === "main";
dragRegion.addEventListener("mousedown", (event) => {
  if (event.button === 0) currentWindow.startDragging();
});

function showMessage(text, error = false) {
  message.textContent = text;
  message.classList.toggle("error", error);
}

async function refresh() {
  const value = await invoke("proof_status");
  status.textContent = JSON.stringify(value, null, 2);
  return value;
}

const actions = {
  refresh,
  "toggle-maximized": () => invoke("toggle_maximized"),
  "create-workspace": () => invoke("set_workspace", { enabled: true }),
  "close-workspace": () => invoke("set_workspace", { enabled: false }),
  "protected-primary": () => invoke("prove_protected_primary"),
  "missing-display": () => invoke("prepare_missing_display_restart"),
  flush: () => invoke("flush_proof"),
  quit: () => invoke("quit_proof"),
};

document.addEventListener("click", async (event) => {
  const button = event.target.closest("button[data-command]");
  if (!button) return;
  button.disabled = true;
  showMessage(`Running ${button.dataset.command}…`);
  try {
    const result = await actions[button.dataset.command]();
    showMessage(
      typeof result === "string" ? result : `${button.dataset.command} complete`,
    );
    await refresh();
  } catch (error) {
    showMessage(String(error), true);
  } finally {
    button.disabled = false;
  }
});

try {
  const receipt = await invoke("page_ready", { label });
  let runtime = await refresh();
  for (
    let attempt = 0;
    label === "main" &&
    !runtime.host.initial_restore_complete &&
    attempt < 12;
    attempt += 1
  ) {
    await new Promise((resolve) => setTimeout(resolve, 100));
    runtime = await refresh();
  }
  showMessage(
    runtime.host.initial_restore_complete || label !== "main"
      ? `Page ready; Longhorn reveal gate complete (${receipt.status.kind}).`
      : "Page ready; placement convergence is still pending.",
  );
} catch (error) {
  status.textContent = String(error);
  showMessage("Page-ready handshake failed.", true);
}
