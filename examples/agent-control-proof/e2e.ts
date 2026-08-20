// Packaged end-to-end driver for Cards 232-234 and 240: an MCP client
// drives the proof app unfocused through snapshot, click, type, wait_for,
// screenshot, and command; two clients interleave; listen streams stay
// isolated; then the opted-in `preview` island is driven the same way,
// with closed-child and cross-webview refusal legs.
//
// Usage (from the repo root, on the operator's display):
//
//   bun examples/agent-control-proof/e2e.ts

import { access, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "../..");
const exampleRoot = join(repoRoot, "examples", "agent-control-proof");
const evidenceRoot = join(exampleRoot, "evidence");
const appPath = join(
  repoRoot,
  "target",
  "release",
  "bundle",
  "macos",
  "Longhorn Agent Control Proof.app",
);
const discoveryDir = join(
  homedir(),
  "Library",
  "Application Support",
  "longhorn",
  "state",
  "agent-control",
);
const appId = "dev.example.longhorn-agent-control-proof";

type Discovery = { appId: string; pid: number; port: number; token: string };
type SemanticNode = {
  elementRef: string;
  role: string;
  name?: string;
  children?: SemanticNode[];
};

async function run(command: readonly string[], cwd?: string): Promise<string> {
  const subprocess = Bun.spawn(command, {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    subprocess.exited,
    new Response(subprocess.stdout).text(),
    new Response(subprocess.stderr).text(),
  ]);
  if (exitCode !== 0) {
    throw new Error(`${command.join(" ")} failed\n${stdout}\n${stderr}`);
  }
  return stdout.trim();
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
}

function meta() {
  return {
    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
    "io.modelcontextprotocol/clientInfo": { name: "agent-control-e2e", version: "0.0.0" },
    "io.modelcontextprotocol/clientCapabilities": {},
  };
}

async function mcp(
  discovery: Discovery,
  method: string,
  params: Record<string, unknown>,
  name?: string,
): Promise<unknown> {
  const headers: Record<string, string> = {
    "content-type": "application/json",
    accept: "application/json, text/event-stream",
    "mcp-protocol-version": "2026-07-28",
    "mcp-method": method,
    authorization: `Bearer ${discovery.token}`,
  };
  if (name) headers["mcp-name"] = name;
  const response = await fetch(`http://127.0.0.1:${discovery.port}/mcp`, {
    method: "POST",
    headers,
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
  });
  if (response.status !== 200) {
    throw new Error(`MCP ${method} answered ${response.status}: ${await response.text()}`);
  }
  const body = await response.text();
  const data = body
    .split("\n")
    .find((line) => line.startsWith("data: "))
    ?.slice("data: ".length);
  if (!data) throw new Error(`MCP ${method} returned no SSE data: ${body}`);
  return JSON.parse(data);
}

async function callTool(
  discovery: Discovery,
  tool: string,
  args: Record<string, unknown>,
): Promise<{ isError: boolean; result: Record<string, unknown> }> {
  const payload = (await mcp(
    discovery,
    "tools/call",
    { name: tool, arguments: args, _meta: meta() },
    tool,
  )) as { result: { isError?: boolean; content: Array<Record<string, unknown>> } };
  const result = payload.result;
  const first = result.content[0] ?? {};
  const parsed =
    first.type === "text" ? (JSON.parse(String(first.text)) as Record<string, unknown>) : first;
  return { isError: result.isError === true, result: parsed };
}

function requireOk(
  tool: string,
  outcome: { isError: boolean; result: Record<string, unknown> },
): Record<string, unknown> {
  if (outcome.isError) {
    throw new Error(`${tool} failed: ${JSON.stringify(outcome.result)}`);
  }
  return outcome.result;
}

function findByName(node: SemanticNode, name: string): SemanticNode | undefined {
  if (node.name === name) return node;
  for (const child of node.children ?? []) {
    const match = findByName(child, name);
    if (match) return match;
  }
  return undefined;
}

async function snapshotRoot(
  discovery: Discovery,
  webview?: string,
): Promise<{ root: SemanticNode; webview?: string }> {
  const args: Record<string, unknown> = webview ? { webview } : {};
  const result = requireOk("snapshot", await callTool(discovery, "snapshot", args));
  return {
    root: result.root as SemanticNode,
    webview: typeof result.webview === "string" ? result.webview : undefined,
  };
}

function pidAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function waitForDiscovery(): Promise<{ discovery: Discovery; path: string }> {
  const deadline = Date.now() + 30_000;
  for (;;) {
    const entries = await readdir(discoveryDir).catch(() => [] as string[]);
    for (const entry of entries) {
      if (!entry.startsWith(`${appId}-`)) continue;
      const path = join(discoveryDir, entry);
      const discovery = JSON.parse(await readFile(path, "utf8")) as Discovery;
      if (pidAlive(discovery.pid)) return { discovery, path };
    }
    if (Date.now() > deadline) throw new Error("discovery file never appeared");
    await sleep(200);
  }
}

async function frontmostPid(): Promise<number> {
  const raw = await run([
    "osascript",
    "-e",
    'tell application "System Events" to unix id of first process whose frontmost is true',
  ]);
  return Number.parseInt(raw, 10);
}

async function listenOnce(
  discovery: Discovery,
  uri: string,
  trigger: () => Promise<void>,
): Promise<boolean> {
  const headers: Record<string, string> = {
    "content-type": "application/json",
    accept: "application/json, text/event-stream",
    "mcp-protocol-version": "2026-07-28",
    "mcp-method": "subscriptions/listen",
    authorization: `Bearer ${discovery.token}`,
  };
  const abort = new AbortController();
  const response = await fetch(`http://127.0.0.1:${discovery.port}/mcp`, {
    method: "POST",
    headers,
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: Math.floor(Math.random() * 1000) + 10,
      method: "subscriptions/listen",
      params: {
        notifications: { resourceSubscriptions: [uri] },
        _meta: meta(),
      },
    }),
    signal: abort.signal,
  });
  if (response.status !== 200 || !response.body) {
    throw new Error(`listen answered ${response.status}`);
  }
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buf = "";
  let sawUpdate = false;
  const deadline = Date.now() + 4_000;
  const read = (async () => {
    while (Date.now() < deadline) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      if (buf.includes("notifications/resources/updated") && buf.includes(uri)) {
        sawUpdate = true;
        break;
      }
    }
  })();
  await sleep(150);
  await trigger();
  await Promise.race([read, sleep(3_500)]);
  abort.abort();
  return sawUpdate;
}

async function main(): Promise<void> {
  const stamp = new Date().toISOString().replaceAll(":", "-").replace(/\..+$/, "");
  const outDir = join(evidenceRoot, `${stamp}-e2e`);
  await mkdir(outDir, { recursive: true });

  console.log("building packaged proof app");
  await run(["bunx", "@tauri-apps/cli", "build"], exampleRoot);

  await run(["pkill", "-f", "Longhorn Agent Control Proof"]).catch(() => {});
  await sleep(500);
  const leftovers = await readdir(discoveryDir).catch(() => [] as string[]);
  for (const entry of leftovers) {
    if (!entry.startsWith(`${appId}-`)) continue;
    const leftover = join(discoveryDir, entry);
    const record = JSON.parse(await readFile(leftover, "utf8")) as Discovery;
    if (!pidAlive(record.pid)) await rm(leftover).catch(() => {});
  }
  // Launch without requiring the app to stay frontmost; immediately hand
  // focus to Finder so the rest of the run is unfocused (contract 022).
  await run(["open", "-g", "-a", appPath]);
  const { discovery, path: discoveryPath } = await waitForDiscovery();
  await run(["osascript", "-e", 'tell application "Finder" to activate']);
  await sleep(300);

  const focusSamples: Array<{ at: string; frontmostPid: number; appHoldsFocus: boolean }> = [];
  async function sampleFocus(at: string): Promise<void> {
    const pid = await frontmostPid();
    const appHoldsFocus = pid === discovery.pid;
    focusSamples.push({ at, frontmostPid: pid, appHoldsFocus });
    if (appHoldsFocus) {
      throw new Error(`app held OS focus at ${at} (pid ${pid})`);
    }
  }

  await sampleFocus("after-unfocus");

  const { root } = await snapshotRoot(discovery);
  const item = findByName(root, "Item");
  const add = findByName(root, "Add");
  if (!item || !add) {
    throw new Error(`missing form controls in snapshot: ${JSON.stringify(root)}`);
  }
  requireOk("click", await callTool(discovery, "click", { element: item.elementRef }));
  requireOk("type", await callTool(discovery, "type", { element: item.elementRef, text: "Alpha" }));
  const typed = requireOk(
    "evaluate",
    await callTool(discovery, "evaluate", {
      js: "document.getElementById('item-name') && document.getElementById('item-name').value",
    }),
  );
  requireOk("click", await callTool(discovery, "click", { element: add.elementRef }));
  const alphaDeadline = Date.now() + 2_000;
  let afterAlpha = (await snapshotRoot(discovery)).root;
  while (!findByName(afterAlpha, "Alpha") && Date.now() < alphaDeadline) {
    await sleep(50);
    afterAlpha = (await snapshotRoot(discovery)).root;
  }
  if (!findByName(afterAlpha, "Alpha")) {
    const names: string[] = [];
    const walk = (node: SemanticNode) => {
      if (node.name) names.push(node.name);
      (node.children ?? []).forEach(walk);
    };
    walk(afterAlpha);
    throw new Error(
      `typed item Alpha did not appear in the list; input value=${JSON.stringify(typed)}; names=${JSON.stringify(names)}`,
    );
  }
  requireOk("type", await callTool(discovery, "type", { element: item.elementRef, text: "Beta" }));
  requireOk("click", await callTool(discovery, "click", { element: add.elementRef }));
  await sleep(100);
  const withBoth = (await snapshotRoot(discovery)).root;
  const alpha = findByName(withBoth, "Alpha");
  const beta = findByName(withBoth, "Beta");
  if (!alpha || !beta) throw new Error("list missing Alpha/Beta after type+click");
  requireOk(
    "drag",
    await callTool(discovery, "drag", { source: alpha.elementRef, target: beta.elementRef }),
  );

  const about = findByName(withBoth, "About");
  if (!about) throw new Error("About link missing");
  requireOk("click", await callTool(discovery, "click", { element: about.elementRef }));
  requireOk(
    "wait_for",
    await callTool(discovery, "wait_for", {
      predicate: { predicate: "pageUrlContains", needle: "#about" },
      timeoutMs: 2_000,
    }),
  );
  await sampleFocus("after-navigation");

  const shotPath = join(outDir, "unfocused-about.png");
  const shot = (await mcp(
    discovery,
    "tools/call",
    { name: "screenshot", arguments: {}, _meta: meta() },
    "screenshot",
  )) as { result: { isError?: boolean; content: Array<Record<string, unknown>> } };
  if (shot.result.isError) throw new Error(`screenshot failed: ${JSON.stringify(shot.result)}`);
  const image = shot.result.content[0];
  if (!image || image.type !== "image") throw new Error("screenshot returned no image");
  await writeFile(shotPath, Buffer.from(String(image.data), "base64"));

  requireOk("command", await callTool(discovery, "command", { command: "proof:ping" }));

  const [left, right] = await Promise.all([snapshotRoot(discovery), snapshotRoot(discovery)]);
  const leftAbout = findByName(left.root, "About");
  const rightAbout = findByName(right.root, "About");
  if (!leftAbout || !rightAbout || leftAbout.elementRef !== rightAbout.elementRef) {
    throw new Error("interleaved snapshots did not share the About ref");
  }

  const consoleUri = "longhorn://agent-control/console";
  const [listenA, listenB] = await Promise.all([
    listenOnce(discovery, consoleUri, async () => {
      requireOk(
        "evaluate",
        await callTool(discovery, "evaluate", { js: "console.log('client-a-probe')" }),
      );
    }),
    listenOnce(discovery, consoleUri, async () => {
      requireOk(
        "evaluate",
        await callTool(discovery, "evaluate", { js: "console.log('client-b-probe')" }),
      );
    }),
  ]);
  if (!listenA || !listenB) {
    throw new Error(`listen streams missed console events a=${listenA} b=${listenB}`);
  }
  await sampleFocus("after-listen");

  // Card 240: drive the opted-in `preview` island, then the closed-child
  // and cross-webview refusal legs, then UI/island interleave.
  const islandSnap = await snapshotRoot(discovery, "preview");
  if (islandSnap.webview !== "preview") {
    throw new Error(`island snapshot did not name preview: ${JSON.stringify(islandSnap.webview)}`);
  }
  const islandGo = findByName(islandSnap.root, "Island Go");
  const islandNote = findByName(islandSnap.root, "Island Note");
  const cellStart = findByName(islandSnap.root, "Cell 0 0");
  const cellEnd = findByName(islandSnap.root, "Cell 2 2");
  if (!islandGo || !islandNote || !cellStart || !cellEnd) {
    throw new Error(`island snapshot missing controls: ${JSON.stringify(islandSnap.root)}`);
  }
  if (!islandGo.elementRef.includes("preview:")) {
    throw new Error(`island ref was not namespaced: ${islandGo.elementRef}`);
  }
  requireOk(
    "click",
    await callTool(discovery, "click", { webview: "preview", element: islandGo.elementRef }),
  );
  requireOk(
    "type",
    await callTool(discovery, "type", {
      webview: "preview",
      element: islandNote.elementRef,
      text: "Marquee",
    }),
  );
  requireOk(
    "drag",
    await callTool(discovery, "drag", {
      webview: "preview",
      source: cellStart.elementRef,
      target: cellEnd.elementRef,
    }),
  );
  requireOk(
    "wait_for",
    await callTool(discovery, "wait_for", {
      webview: "preview",
      predicate: { predicate: "pageTitleContains", needle: "Ready" },
      timeoutMs: 2_000,
    }),
  );
  const islandEval = requireOk(
    "evaluate",
    await callTool(discovery, "evaluate", {
      webview: "preview",
      js: "JSON.stringify({ note: document.getElementById('island-note') && document.getElementById('island-note').value, selection: document.getElementById('selection') && document.getElementById('selection').textContent, title: document.title })",
    }),
  );
  const islandState =
    typeof islandEval.value === "string" ? JSON.parse(islandEval.value) : islandEval.value;
  if (islandState.note !== "Marquee") {
    throw new Error(`island type did not land: ${JSON.stringify(islandState)}`);
  }
  if (islandState.selection !== "0,0:2,2") {
    throw new Error(`island drag did not select 0,0:2,2: ${JSON.stringify(islandState)}`);
  }
  if (!String(islandState.title).includes("Ready")) {
    throw new Error(`island click did not retitle: ${JSON.stringify(islandState)}`);
  }
  await sampleFocus("after-island-drive");

  const islandShotPath = join(outDir, "unfocused-island.png");
  const islandShot = (await mcp(
    discovery,
    "tools/call",
    { name: "screenshot", arguments: {}, _meta: meta() },
    "screenshot",
  )) as { result: { isError?: boolean; content: Array<Record<string, unknown>> } };
  if (islandShot.result.isError) {
    throw new Error(`island screenshot failed: ${JSON.stringify(islandShot.result)}`);
  }
  const islandImage = islandShot.result.content[0];
  if (!islandImage || islandImage.type !== "image") {
    throw new Error("island screenshot returned no image");
  }
  await writeFile(islandShotPath, Buffer.from(String(islandImage.data), "base64"));

  const closed = await callTool(discovery, "snapshot", { webview: "preview-top" });
  if (!closed.isError || closed.result.error !== "unsupported") {
    throw new Error(`closed island must answer Unsupported: ${JSON.stringify(closed)}`);
  }
  if (!String(closed.result.message).includes("not opted in")) {
    throw new Error(`closed island refusal must name opt-in absence: ${JSON.stringify(closed)}`);
  }
  const cross = await callTool(discovery, "click", { element: islandGo.elementRef });
  if (!cross.isError || cross.result.error !== "unresolvedRef") {
    throw new Error(`cross-webview ref must be UnresolvedRef: ${JSON.stringify(cross)}`);
  }

  const [uiClient, islandClient] = await Promise.all([
    snapshotRoot(discovery),
    snapshotRoot(discovery, "preview"),
  ]);
  const uiAbout = findByName(uiClient.root, "About");
  const islandGoAgain = findByName(islandClient.root, "Island Go");
  if (!uiAbout || !islandGoAgain) {
    throw new Error("interleave snapshots lost UI About or island Go");
  }
  if (uiAbout.elementRef === islandGoAgain.elementRef) {
    throw new Error("UI and island refs collided");
  }
  requireOk("click", await callTool(discovery, "click", { element: uiAbout.elementRef }));
  requireOk(
    "click",
    await callTool(discovery, "click", {
      webview: "preview",
      element: islandGoAgain.elementRef,
    }),
  );
  await sampleFocus("after-island-interleave");

  await run(["osascript", "-e", `tell application id "${appId}" to quit`]);
  let discoveryRemoved = false;
  for (let attempt = 0; attempt < 50; attempt += 1) {
    const gone = await access(discoveryPath).then(() => false).catch(() => true);
    if (gone) {
      discoveryRemoved = true;
      break;
    }
    await sleep(200);
  }

  const receipt = {
    schema: "longhorn.agent-control-e2e.v2",
    app: appPath,
    pid: discovery.pid,
    discoveryPath,
    screenshot: shotPath,
    islandScreenshot: islandShotPath,
    focusSamples,
    appHeldFocus: focusSamples.some((sample) => sample.appHoldsFocus),
    twoClientRefsShared: leftAbout.elementRef === rightAbout.elementRef,
    listenA,
    listenB,
    island: {
      typedNote: islandState.note,
      selection: islandState.selection,
      title: islandState.title,
      closedChildUnsupported: closed.result.error === "unsupported",
      crossWebviewUnresolved: cross.result.error === "unresolvedRef",
      uiIslandRefsDistinct: uiAbout.elementRef !== islandGoAgain.elementRef,
    },
    discoveryRemovedOnQuit: discoveryRemoved,
    osPointerUsed: false,
  };
  await writeFile(join(outDir, "e2e.json"), `${JSON.stringify(receipt, null, 2)}\n`);
  console.log(JSON.stringify(receipt, null, 2));
  if (
    receipt.appHeldFocus ||
    !discoveryRemoved ||
    !listenA ||
    !listenB ||
    !receipt.island.closedChildUnsupported ||
    !receipt.island.crossWebviewUnresolved ||
    !receipt.island.uiIslandRefsDistinct ||
    receipt.island.selection !== "0,0:2,2" ||
    receipt.island.typedNote !== "Marquee"
  ) {
    throw new Error(`e2e failed; evidence in ${outDir}`);
  }
  console.log(`e2e passed; evidence in ${outDir}`);
}

await main();
