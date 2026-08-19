// Card 237 dogfood: follow the skill, not the Card 234 e2e driver.
// Launch is the guide's worked example. Driving steps come only from
// skills/agent-control/SKILL.md (finder, raw-POST fallback, tool order).

import { mkdir, readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "../..");
const finderPath = join(repoRoot, "skills", "agent-control", "scripts", "find-instance.ts");
const evidenceRoot = join(import.meta.dir, "evidence");
const appPath = join(
  repoRoot,
  "target",
  "release",
  "bundle",
  "macos",
  "Longhorn Agent Control Proof.app",
);

type Discovery = { appId: string; pid: number; port: number; token: string };
type SemanticNode = {
  elementRef: string;
  role: string;
  name?: string;
  children?: SemanticNode[];
};

const findings: string[] = [];

function note(finding: string): void {
  findings.push(finding);
}

function redact(text: string, token: string): string {
  return text.split(token).join("<redacted-token>");
}

async function run(command: readonly string[], cwd?: string): Promise<{
  code: number;
  stdout: string;
  stderr: string;
}> {
  const subprocess = Bun.spawn(command, { cwd, stdout: "pipe", stderr: "pipe" });
  const [code, stdout, stderr] = await Promise.all([
    subprocess.exited,
    new Response(subprocess.stdout).text(),
    new Response(subprocess.stderr).text(),
  ]);
  return { code, stdout, stderr };
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
}

function meta() {
  return {
    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
    "io.modelcontextprotocol/clientInfo": { name: "agent-control", version: "0.1.0" },
    "io.modelcontextprotocol/clientCapabilities": {},
  };
}

async function mcp(
  discovery: Discovery,
  method: string,
  params: Record<string, unknown>,
  name?: string,
): Promise<{ status: number; body: string; json: unknown }> {
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
  const body = await response.text();
  const data = body
    .split("\n")
    .find((line) => line.startsWith("data: "))
    ?.slice("data: ".length);
  return {
    status: response.status,
    body,
    json: data ? JSON.parse(data) : null,
  };
}

async function callTool(
  discovery: Discovery,
  tool: string,
  args: Record<string, unknown>,
): Promise<{ isError: boolean; result: Record<string, unknown>; raw: unknown }> {
  const reply = await mcp(
    discovery,
    "tools/call",
    { name: tool, arguments: args, _meta: meta() },
    tool,
  );
  const payload = reply.json as {
    result?: { isError?: boolean; content: Array<Record<string, unknown>> };
  };
  const first = payload.result?.content[0] ?? {};
  const parsed =
    first.type === "text" ? (JSON.parse(String(first.text)) as Record<string, unknown>) : first;
  return { isError: payload.result?.isError === true, result: parsed, raw: reply.json };
}

function findByName(node: SemanticNode, name: string): SemanticNode | undefined {
  if (node.name === name) return node;
  for (const child of node.children ?? []) {
    const match = findByName(child, name);
    if (match) return match;
  }
  return undefined;
}

async function frontmostPid(): Promise<number> {
  const result = await run([
    "osascript",
    "-e",
    'tell application "System Events" to unix id of first process whose frontmost is true',
  ]);
  return Number.parseInt(result.stdout.trim(), 10);
}

async function main(): Promise<void> {
  const stamp = new Date().toISOString().replaceAll(":", "-").replace(/\..+$/, "");
  const outDir = join(evidenceRoot, `${stamp}-skill-dogfood`);
  await mkdir(outDir, { recursive: true });

  const commands: Array<Record<string, unknown>> = [];
  const toolCalls: Array<Record<string, unknown>> = [];
  const focusSamples: Array<{ at: string; frontmostPid: number; appHoldsFocus: boolean }> = [];

  const record = (label: string, command: readonly string[], result: { code: number; stdout: string; stderr: string }, token?: string) => {
    commands.push({
      label,
      command,
      code: result.code,
      stdout: token ? redact(result.stdout, token) : result.stdout,
      stderr: token ? redact(result.stderr, token) : result.stderr,
    });
  };

  // Guide: launch the packaged proof unfocused.
  const launch = await run(["open", "-g", "-a", appPath]);
  record("guide: open -g packaged proof", ["open", "-g", "-a", appPath], launch);
  if (launch.code !== 0) {
    throw new Error(`launch failed: ${launch.stderr}`);
  }

  let finder = { code: 1, stdout: "", stderr: "" };
  for (let attempt = 0; attempt < 40; attempt += 1) {
    finder = await run(["bun", finderPath, "--app-id", "dev.example.longhorn-agent-control-proof"]);
    if (finder.code === 0) break;
    await sleep(250);
  }
  record("skill: finder", ["bun", "skills/agent-control/scripts/find-instance.ts", "--app-id", "dev.example.longhorn-agent-control-proof"], finder);
  if (finder.code !== 0) {
    throw new Error(`finder never found a live instance: ${finder.stderr}`);
  }
  if (finder.stderr.toLowerCase().includes("bearer") || /Authorization: Bearer \S+/.test(finder.stderr)) {
    note("finder diagnostics contained a token — skill/script contract broken");
  }

  const urlMatch = finder.stdout.match(/url: (http:\/\/127\.0\.0\.1:\d+\/mcp)/);
  const paste = finder.stdout.match(/claude mcp add --transport http \S+ http:\/\/127\.0\.0\.1:(\d+)\/mcp --header "Authorization: Bearer (\S+)"/);
  if (!urlMatch || !paste) {
    note("finder stdout was not parseable as the skill's url + claude mcp add line");
    throw new Error(`finder stdout not parseable:\n${finder.stdout}`);
  }
  const discoveryDir = join(homedir(), "Library", "Application Support", "longhorn", "state", "agent-control");
  const pidMatch = finder.stdout.match(/longhorn-dev\.example\.longhorn-agent-control-proof-(\d+)/);
  const discovery: Discovery = {
    appId: "dev.example.longhorn-agent-control-proof",
    pid: Number.parseInt(pidMatch?.[1] ?? "0", 10),
    port: Number.parseInt(paste[1], 10),
    token: paste[2],
  };
  if (!discovery.pid) {
    // Recover pid from the live discovery file named in the finder diagnostics.
    const files = finder.stderr;
    void files;
    const listing = await run(["bun", "-e", `const {readdir,readFile}=require('fs/promises'); const dir=${JSON.stringify(discoveryDir)}; const names=await readdir(dir); for (const n of names) { if (!n.startsWith('dev.example.longhorn-agent-control-proof-')) continue; const j=JSON.parse(await readFile(dir+'/'+n,'utf8')); if (j.port===${discovery.port}) { console.log(j.pid); break; } }`]);
    discovery.pid = Number.parseInt(listing.stdout.trim(), 10);
  }

  await run(["osascript", "-e", 'tell application "Finder" to activate']);
  await sleep(300);

  async function sample(at: string): Promise<void> {
    const pid = await frontmostPid();
    const appHoldsFocus = pid === discovery.pid;
    focusSamples.push({ at, frontmostPid: pid, appHoldsFocus });
    if (appHoldsFocus) {
      throw new Error(`app held OS focus at ${at}`);
    }
  }
  await sample("after-unfocus");

  const snapshot = await callTool(discovery, "snapshot", {});
  toolCalls.push({ tool: "snapshot", args: {}, isError: snapshot.isError, result: snapshot.result });
  if (snapshot.isError) throw new Error(`snapshot failed: ${JSON.stringify(snapshot.result)}`);
  const root = snapshot.result.root as SemanticNode;
  const item = findByName(root, "Item");
  const add = findByName(root, "Add");
  if (!item || !add) {
    note("snapshot did not expose named Item/Add controls; skill says walk by role/name");
    throw new Error("missing Item/Add in snapshot");
  }

  const clickItem = await callTool(discovery, "click", { element: item.elementRef });
  toolCalls.push({ tool: "click", args: { element: "<item-ref>" }, isError: clickItem.isError, result: clickItem.result });
  if (clickItem.isError) throw new Error(`click Item failed: ${JSON.stringify(clickItem.result)}`);

  const typed = await callTool(discovery, "type", { element: item.elementRef, text: "Dogfood" });
  toolCalls.push({ tool: "type", args: { element: "<item-ref>", text: "Dogfood" }, isError: typed.isError, result: typed.result });
  if (typed.isError) throw new Error(`type failed: ${JSON.stringify(typed.result)}`);

  const clickAdd = await callTool(discovery, "click", { element: add.elementRef });
  toolCalls.push({ tool: "click", args: { element: "<add-ref>" }, isError: clickAdd.isError, result: clickAdd.result });
  if (clickAdd.isError) throw new Error(`click Add failed: ${JSON.stringify(clickAdd.result)}`);

  const about = findByName(root, "About");
  if (!about) {
    note("About link missing from first snapshot");
    throw new Error("About link missing");
  }
  const clickAbout = await callTool(discovery, "click", { element: about.elementRef });
  toolCalls.push({ tool: "click", args: { element: "<about-ref>" }, isError: clickAbout.isError, result: clickAbout.result });
  if (clickAbout.isError) throw new Error(`click About failed: ${JSON.stringify(clickAbout.result)}`);

  const waited = await callTool(discovery, "wait_for", {
    predicate: { predicate: "pageUrlContains", needle: "about" },
    timeoutMs: 2000,
  });
  toolCalls.push({
    tool: "wait_for",
    args: { predicate: { predicate: "pageUrlContains", needle: "about" }, timeoutMs: 2000 },
    isError: waited.isError,
    result: waited.result,
  });
  if (waited.isError) throw new Error(`wait_for failed: ${JSON.stringify(waited.result)}`);
  await sample("after-wait_for");

  const shot = await mcp(discovery, "tools/call", { name: "screenshot", arguments: {}, _meta: meta() }, "screenshot");
  const shotPayload = shot.json as {
    result?: { isError?: boolean; content: Array<Record<string, unknown>> };
  };
  if (shotPayload.result?.isError) {
    throw new Error(`screenshot failed: ${JSON.stringify(shotPayload.result)}`);
  }
  const image = shotPayload.result?.content[0];
  if (!image || image.type !== "image") {
    note("screenshot did not return image content as the skill describes");
    throw new Error("screenshot returned no image");
  }
  const shotPath = join(outDir, "unfocused-about.png");
  await writeFile(shotPath, Buffer.from(String(image.data), "base64"));
  toolCalls.push({ tool: "screenshot", args: {}, isError: false, result: { type: "image", path: "unfocused-about.png" } });

  const ping = await callTool(discovery, "command", { command: "proof:ping" });
  toolCalls.push({ tool: "command", args: { command: "proof:ping" }, isError: ping.isError, result: ping.result });
  if (ping.isError) throw new Error(`command proof:ping failed: ${JSON.stringify(ping.result)}`);

  const consoleUri = "longhorn://agent-control/console";
  const abort = new AbortController();
  const listenResponse = await fetch(`http://127.0.0.1:${discovery.port}/mcp`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      accept: "application/json, text/event-stream",
      "mcp-protocol-version": "2026-07-28",
      "mcp-method": "subscriptions/listen",
      authorization: `Bearer ${discovery.token}`,
    },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 10,
      method: "subscriptions/listen",
      params: {
        notifications: { resourceSubscriptions: [consoleUri] },
        _meta: meta(),
      },
    }),
    signal: abort.signal,
  });
  if (listenResponse.status !== 200 || !listenResponse.body) {
    throw new Error(`listen answered ${listenResponse.status}: ${await listenResponse.text()}`);
  }
  const reader = listenResponse.body.getReader();
  const decoder = new TextDecoder();
  let buf = "";
  let sawAck = false;
  let sawUpdate = false;
  const read = (async () => {
    const deadline = Date.now() + 8_000;
    while (Date.now() < deadline && !sawUpdate) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      if (buf.includes("notifications/subscriptions/acknowledged")) sawAck = true;
      if (buf.includes("notifications/resources/updated") && buf.includes(consoleUri)) {
        sawUpdate = true;
        break;
      }
    }
  })();
  await sleep(200);
  const logged = await callTool(discovery, "evaluate", { js: "console.log('dogfood-listen')" });
  toolCalls.push({
    tool: "evaluate",
    args: { js: "console.log('dogfood-listen')" },
    isError: logged.isError,
    result: logged.result,
  });
  await Promise.race([read, sleep(7_000)]);
  abort.abort();
  const readResource = await mcp(
    discovery,
    "resources/read",
    { uri: consoleUri, _meta: meta() },
    consoleUri,
  );
  const resourceBody = readResource.json;
  toolCalls.push({
    tool: "subscriptions/listen",
    args: { resourceSubscriptions: [consoleUri] },
    sawAck,
    sawUpdate,
    excerpt: redact(buf.slice(0, 800), discovery.token),
    resourceRead: resourceBody,
  });
  if (!sawAck) {
    note("listen stream never sent subscriptions/acknowledged");
  }
  if (!sawUpdate) {
    note("listen stream did not deliver resources/updated for console after evaluate console.log");
    throw new Error(`listen missed console update; buf=${buf.slice(0, 400)}`);
  }
  await sample("after-listen");

  const quit = await run(["osascript", "-e", 'tell application id "dev.example.longhorn-agent-control-proof" to quit']);
  record("quit proof app", ["osascript", "-e", 'tell application id "dev.example.longhorn-agent-control-proof" to quit'], quit);

  const receipt = {
    schema: "longhorn.agent-control-skill-dogfood.v1",
    app: appPath,
    pid: discovery.pid,
    url: urlMatch[1],
    finder: {
      stdout: redact(finder.stdout, discovery.token),
      stderr: redact(finder.stderr, discovery.token),
    },
    commands,
    toolCalls,
    focusSamples,
    appHeldFocus: focusSamples.some((sample) => sample.appHoldsFocus),
    osPointerUsed: false,
    findings,
    screenshot: "unfocused-about.png",
  };
  const json = redact(`${JSON.stringify(receipt, null, 2)}\n`, discovery.token);
  if (json.includes(discovery.token)) {
    throw new Error("token leaked into committed evidence");
  }
  await writeFile(join(outDir, "dogfood.json"), json);
  console.log(json);
  console.log(JSON.stringify({ outDir, appHeldFocus: receipt.appHeldFocus, findings }, null, 2));
  if (receipt.appHeldFocus || findings.length > 0) {
    throw new Error(`dogfood failed; evidence in ${outDir}`);
  }
  console.log(`dogfood passed; evidence in ${outDir}`);
}

await main();
