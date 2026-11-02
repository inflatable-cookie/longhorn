// Proof behavior for the bundled offline document.
//
// Every remote URL here is a deliberate denial probe: the host policy must
// deny the popup and the navigation, and the unpermitted command must be
// rejected by the capability scope. The document's final `document.title` is
// the one evidence channel back to the host process — the child holds exactly
// one IPC permission, so nothing else can report. The host asserts the exact
// title shape, so every check below must resolve before the title is set.
"use strict";

const ESCAPE_POPUP_URL = "https://escape.invalid/popup";
const ESCAPE_NAVIGATION_URL = "https://escape.invalid/escape";
const SETTLE_MS = 600;

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function run() {
  const parts = [];

  // 1. Unintended popup creation must be denied closed.
  const popup = window.open(ESCAPE_POPUP_URL, "_blank");
  parts.push(popup === null ? "popup:denied" : "popup:ALLOWED");

  // 2. Scoped IPC: the one permitted command echoes the host-injected nonce.
  try {
    const result = await window.__TAURI__.core.invoke("ipc_proof_ping", {
      nonce: window.__NCL_PROOF_NONCE,
    });
    parts.push("ping:" + result);
  } catch (error) {
    parts.push("ping:FAILED:" + String(error));
  }

  // 3. IPC scope: a command that exists but is not permitted must be denied.
  try {
    await window.__TAURI__.core.invoke("ipc_forbidden_probe", {});
    parts.push("forbidden:ALLOWED");
  } catch (error) {
    parts.push("forbidden:denied");
  }

  // 4. Unintended navigation must be denied. If the escape navigation were
  // admitted, this script context would be destroyed and the final title
  // would never arrive, failing the proof from the host side (watchdog).
  window.location.href = ESCAPE_NAVIGATION_URL;
  await sleep(SETTLE_MS);
  const stillHere = window.location.pathname.endsWith("/offline/document.html");
  parts.push(stillHere ? "navigation:denied" : "navigation:LOST");

  // 5. The single evidence channel.
  document.title = "ncl-proof|" + parts.join("|");
}

run();
