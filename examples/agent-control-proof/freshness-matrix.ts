// Drives the Card 231 packaged freshness matrix against the bundled
// agent-control proof app (contract 022 evidence: unfocused, occluded, and
// minimized screenshots are fresh, judged DOM-relative — never wall-clock).
//
// The spike's matrix judged freshness by reading the counter off the PNG
// against `evaluate` brackets. This driver automates the same judgment:
// the page encodes its counter in the background hue (stride 47°, so
// adjacent seconds are visually distinct), each screenshot is bracketed by
// `evaluate` reads of the counter, and the captured pixels must match one
// of the bracketed counters' hues. The big numeral remains for human
// spot-checks; PNGs land in `evidence/` next to the matrix receipt.
//
// Card 238 adds the preview islands: child webviews attached to the main
// window — a base island (97° stride, oversized so it clips right and
// bottom), an overlapping island attached after it (199° stride; the
// overlap region must name it), and a hidden island (its region must show
// the parent page). The islands are not semantic targets, so their
// counters cannot be bracketed by `evaluate` directly; instead all tickers
// start at page load and tick at 1 Hz, so an island counter stays within a
// couple of ticks of the parent's bracket — island pixels are judged
// against island hues for counters in [before-2, after+2]. A pre-fix run
// shows the island region carrying the parent's pixels (no island hue
// match): that failure is the baseline fixture. A frontmost-only geometry
// probe checks the base island's left and top edges pixel-exactly, the
// overlap order, and the hidden island's absence, and records the PNG
// dimensions so the observed scale factor is on record.
//
// Window states are scripted without accessibility permissions: focus and
// the covering Terminal window go through AppleScript (Terminal's own
// dictionary, plus the DOM's window.screenX/Y for placement), minimize and
// restore go through the app's own contract-006 commands over the MCP
// `command` tool — the same path an agent would use.
//
// Usage (from the repo root, on the operator's display):
//
//   bunx @tauri-apps/cli build        # from examples/agent-control-proof
//   bun examples/agent-control-proof/freshness-matrix.ts

import { mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
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

type MatrixRow = {
  state: string;
  bracketBefore: number;
  bracketAfter: number;
  matchedCounter: number | null;
  pixel: [number, number, number];
  childMatchedCounter: number | null;
  childPixel: [number, number, number];
  fresh: boolean;
};

async function run(command: readonly string[]): Promise<string> {
  const subprocess = Bun.spawn(command, { stdout: "pipe", stderr: "pipe" });
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

async function waitForDiscovery(): Promise<{ discovery: Discovery; path: string }> {
  const deadline = Date.now() + 30_000;
  for (;;) {
    const entries = await readdir(discoveryDir).catch(() => [] as string[]);
    const match = entries.find((entry) => entry.startsWith(`${appId}-`));
    if (match) {
      const path = join(discoveryDir, match);
      const discovery = JSON.parse(await readFile(path, "utf8")) as Discovery;
      return { discovery, path };
    }
    if (Date.now() > deadline) throw new Error("discovery file never appeared");
    await sleep(200);
  }
}

// Minimal streamable-HTTP MCP client, one POST per call.
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

function meta() {
  return {
    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
    "io.modelcontextprotocol/clientInfo": { name: "freshness-matrix", version: "0.0.0" },
    "io.modelcontextprotocol/clientCapabilities": {},
  };
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
  const isError = result.isError === true;
  const first = result.content[0] ?? {};
  const parsed =
    first.type === "text" ? (JSON.parse(String(first.text)) as Record<string, unknown>) : first;
  return { isError, result: parsed };
}

async function evaluateCounter(discovery: Discovery): Promise<number> {
  const { isError, result } = await callTool(discovery, "evaluate", {
    js: "document.getElementById('counter').textContent",
  });
  if (isError) throw new Error(`evaluate failed: ${JSON.stringify(result)}`);
  return Number.parseInt(String(result.value), 10);
}

async function evaluateJson<T>(discovery: Discovery, js: string): Promise<T> {
  const { isError, result } = await callTool(discovery, "evaluate", { js });
  if (isError) throw new Error(`evaluate failed: ${JSON.stringify(result)}`);
  return JSON.parse(String(result.value)) as T;
}

async function screenshot(discovery: Discovery, path: string): Promise<void> {
  const payload = (await mcp(
    discovery,
    "tools/call",
    { name: "screenshot", arguments: {}, _meta: meta() },
    "screenshot",
  )) as { result: { isError?: boolean; content: Array<Record<string, unknown>> } };
  if (payload.result.isError) {
    throw new Error(`screenshot failed: ${JSON.stringify(payload.result.content)}`);
  }
  const image = payload.result.content[0];
  if (!image || image.type !== "image") {
    throw new Error(`screenshot returned no image content: ${JSON.stringify(payload.result)}`);
  }
  await writeFile(path, Buffer.from(String(image.data), "base64"));
}

// CSS hsl() → sRGB bytes, matching the page's hue encoding.
function hslToRgb(hue: number, saturation: number, lightness: number): [number, number, number] {
  const h = (((hue % 360) + 360) % 360) / 60;
  const c = (1 - Math.abs(2 * lightness - 1)) * saturation;
  const x = c * (1 - Math.abs((h % 2) - 1));
  const [r, g, b] =
    h < 1 ? [c, x, 0]
    : h < 2 ? [x, c, 0]
    : h < 3 ? [0, c, x]
    : h < 4 ? [0, x, c]
    : h < 5 ? [x, 0, c]
    : [c, 0, x];
  const m = lightness - c / 2;
  return [r, g, b].map((channel) => Math.round((channel + m) * 255)) as [
    number,
    number,
    number,
  ];
}

function expectedPixel(counter: number): [number, number, number] {
  return hslToRgb((counter * 47) % 360, 0.7, 0.45);
}

// The island's encoding: 97° stride at 0.55 lightness — pairwise-distinct
// from every parent hue within the judged ranges (28° minimum circular
// separation inside a five-counter scan window is ~75/255 per channel).
function expectedChildPixel(counter: number): [number, number, number] {
  return hslToRgb((counter * 97) % 360, 0.7, 0.55);
}

// The overlapping island's encoding: 199° stride at 0.55 lightness.
function expectedChildTopPixel(counter: number): [number, number, number] {
  return hslToRgb((counter * 199) % 360, 0.7, 0.55);
}

// PNG pixel dimensions straight from the IHDR.
async function pngSize(pngPath: string): Promise<{ width: number; height: number }> {
  const png = await readFile(pngPath);
  return { width: png.readUInt32BE(16), height: png.readUInt32BE(20) };
}

// Reads one pixel out of a PNG via sips → BMP (macOS built-ins only).
async function pixelAt(pngPath: string, fractionX: number, fractionY: number): Promise<[number, number, number]> {
  const bmpPath = pngPath.replace(/\.png$/, ".bmp");
  await run(["sips", "-s", "format", "bmp", pngPath, "--out", bmpPath]);
  const bmp = await readFile(bmpPath);
  const dataOffset = bmp.readUInt32LE(10);
  const width = bmp.readInt32LE(18);
  const rawHeight = bmp.readInt32LE(22);
  const bitsPerPixel = bmp.readUInt16LE(28);
  if (bitsPerPixel !== 24 && bitsPerPixel !== 32) {
    throw new Error(`unexpected BMP depth ${bitsPerPixel}`);
  }
  const bytesPerPixel = bitsPerPixel / 8;
  const rowSize = Math.ceil((width * bytesPerPixel) / 4) * 4;
  const height = Math.abs(rawHeight);
  const x = Math.floor(width * fractionX);
  // Negative height means top-down rows; fractionY counts from the top.
  const y = Math.floor(height * fractionY);
  const row = rawHeight < 0 ? y : height - 1 - y;
  const offset = dataOffset + row * rowSize + x * bytesPerPixel;
  const pixel: [number, number, number] = [bmp[offset + 2]!, bmp[offset + 1]!, bmp[offset]!];
  await rm(bmpPath);
  return pixel;
}

function colorNear(
  actual: [number, number, number],
  expected: [number, number, number],
  tolerance: number,
): boolean {
  return actual.every((channel, index) => Math.abs(channel - expected[index]!) <= tolerance);
}

async function activateApp(): Promise<void> {
  await run(["osascript", "-e", `tell application id "${appId}" to activate`]);
}

async function activateTerminal(): Promise<void> {
  await run(["osascript", "-e", 'tell application "Terminal" to activate']);
}

async function coverWithTerminal(discovery: Discovery): Promise<void> {
  const bounds = await evaluateJson<{ x: number; y: number; w: number; h: number }>(
    discovery,
    "JSON.stringify({x: window.screenX, y: window.screenY, w: window.outerWidth, h: window.outerHeight})",
  );
  await run([
    "osascript",
    "-e",
    'tell application "Terminal" to do script ""',
    "-e",
    `tell application "Terminal" to set bounds of front window to {${bounds.x - 20}, ${bounds.y - 20}, ${bounds.x + bounds.w + 20}, ${bounds.y + bounds.h + 20}}`,
    "-e",
    'tell application "Terminal" to activate',
  ]);
}

async function closeCoveringTerminal(): Promise<void> {
  await run([
    "osascript",
    "-e",
    'tell application "Terminal" to close front window saving no',
  ]).catch(() => {});
}

async function command(discovery: Discovery, id: string): Promise<void> {
  const { isError, result } = await callTool(discovery, "command", { command: id });
  if (isError) throw new Error(`command ${id} failed: ${JSON.stringify(result)}`);
}

async function main(): Promise<void> {
  const stamp = new Date().toISOString().replaceAll(":", "-").replace(/\..+$/, "");
  const outDir = join(evidenceRoot, `${stamp}-packaged`);
  await mkdir(outDir, { recursive: true });

  await run(["pkill", "-f", "Longhorn Agent Control Proof"]).catch(() => {});
  await sleep(500);
  await run(["open", "-a", appPath]);
  const { discovery } = await waitForDiscovery();

  const rows: MatrixRow[] = [];
  async function probe(state: string, apply: () => Promise<void>): Promise<void> {
    await apply();
    await sleep(1600);
    const before = await evaluateCounter(discovery);
    const pngPath = join(outDir, `${state}.png`);
    await screenshot(discovery, pngPath);
    const after = await evaluateCounter(discovery);
    const pixel = await pixelAt(pngPath, 0.25, 0.25);
    let matched: number | null = null;
    for (let counter = before; counter <= after; counter += 1) {
      if (colorNear(pixel, expectedPixel(counter), 12)) matched = counter;
    }
    // The island cannot be bracketed by `evaluate` (not a semantic target);
    // both tickers start at page load and tick at 1 Hz, so its counter sits
    // within a couple of ticks of the parent's bracket. The probe point
    // (0.569, 0.9375) = (410, 450) logical sits low in the island, left of
    // the overlapping second island — the island band is bottom-heavy
    // (y 120..480), so a vertical flip bug lands this probe outside it.
    const childPixel = await pixelAt(pngPath, 0.569, 0.9375);
    let childMatched: number | null = null;
    for (let counter = before - 2; counter <= after + 2; counter += 1) {
      if (colorNear(childPixel, expectedChildPixel(counter), 12)) childMatched = counter;
    }
    rows.push({
      state,
      bracketBefore: before,
      bracketAfter: after,
      matchedCounter: matched,
      pixel,
      childMatchedCounter: childMatched,
      childPixel,
      fresh: matched !== null && childMatched !== null,
    });
    console.log(
      `${state}: bracket ${before}..${after}, pixel [${pixel}], matched ${matched ?? "none"}; island [${childPixel}], matched ${childMatched ?? "none"}`,
    );
  }

  // Frontmost-only geometry probe (Card 238): the island spans logical
  // x 360..760, y 120..520 of the 720x480 window (clipped right and bottom);
  // the second island spans x 460..760, y 220..520 over it; the hidden
  // island covers x 24..224, y 300..450 but shows nothing. Pixels one
  // logical pixel outside/inside the base island's left and top edges pin
  // its placement pixel-exactly; the top-edge pair also catches a
  // vertical-flip bug (a flipped island would invert the two judgments).
  // The overlap pixel names the top island; the hidden region must show the
  // parent page. The PNG's pixel dimensions record the observed scale.
  async function geometryProbe(): Promise<Record<string, unknown>> {
    const before = await evaluateCounter(discovery);
    const pngPath = join(outDir, "geometry.png");
    await screenshot(discovery, pngPath);
    const after = await evaluateCounter(discovery);
    const size = await pngSize(pngPath);
    async function judge(
      fractionX: number,
      fractionY: number,
    ): Promise<{ pixel: [number, number, number]; surface: string; matchedCounter: number | null }> {
      const pixel = await pixelAt(pngPath, fractionX, fractionY);
      let matched: number | null = null;
      for (let counter = before; counter <= after; counter += 1) {
        if (colorNear(pixel, expectedPixel(counter), 12)) matched = counter;
      }
      if (matched !== null) return { pixel, surface: "parent", matchedCounter: matched };
      for (let counter = before - 2; counter <= after + 2; counter += 1) {
        if (colorNear(pixel, expectedChildPixel(counter), 12)) matched = counter;
      }
      if (matched !== null) return { pixel, surface: "island", matchedCounter: matched };
      for (let counter = before - 2; counter <= after + 2; counter += 1) {
        if (colorNear(pixel, expectedChildTopPixel(counter), 12)) matched = counter;
      }
      return { pixel, surface: matched === null ? "neither" : "island-top", matchedCounter: matched };
    }
    const leftOfEdge = await judge(359.5 / 720, 240.5 / 480);
    const rightOfEdge = await judge(360.5 / 720, 240.5 / 480);
    const aboveEdge = await judge(540.5 / 720, 119.5 / 480);
    const belowEdge = await judge(540.5 / 720, 120.5 / 480);
    const overlap = await judge(600.5 / 720, 400.5 / 480);
    const hiddenRegion = await judge(100.5 / 720, 375.5 / 480);
    const pass =
      leftOfEdge.surface === "parent" &&
      rightOfEdge.surface === "island" &&
      aboveEdge.surface === "parent" &&
      belowEdge.surface === "island" &&
      overlap.surface === "island-top" &&
      hiddenRegion.surface === "parent";
    return {
      pngPixels: size,
      windowLogical: { width: 720, height: 480 },
      observedScaleFactor: size.width / 720,
      islandLogicalBounds: { x: 360, y: 120, width: 400, height: 400 },
      islandTopLogicalBounds: { x: 460, y: 220, width: 300, height: 300 },
      islandHiddenLogicalBounds: { x: 24, y: 300, width: 200, height: 150 },
      leftOfEdge,
      rightOfEdge,
      aboveEdge,
      belowEdge,
      overlap,
      hiddenRegion,
      pass,
    };
  }

  await probe("frontmost", activateApp);
  const geometry = await geometryProbe();
  await probe("unfocused", activateTerminal);
  await probe("occluded", () => coverWithTerminal(discovery));
  await closeCoveringTerminal();
  await activateApp();
  await probe("minimized", () => command(discovery, "proof:window.minimize"));
  await probe("restored", async () => {
    await command(discovery, "proof:window.restore");
    await activateApp();
  });

  // Clean exit must remove the discovery file (contract 022 lifecycle).
  await run(["osascript", "-e", `tell application id "${appId}" to quit`]);
  let discoveryRemoved = false;
  for (let attempt = 0; attempt < 50; attempt += 1) {
    const entries = await readdir(discoveryDir).catch(() => [] as string[]);
    if (!entries.some((entry) => entry.startsWith(`${appId}-`))) {
      discoveryRemoved = true;
      break;
    }
    await sleep(200);
  }

  const receipt = {
    schema: "longhorn.agent-control-freshness-matrix.v2",
    app: appPath,
    pid: discovery.pid,
    rows,
    geometry,
    discoveryRemovedOnQuit: discoveryRemoved,
    fresh: rows.every((row) => row.fresh) && (geometry.pass as boolean),
  };
  await writeFile(join(outDir, "matrix.json"), `${JSON.stringify(receipt, null, 2)}\n`);
  console.log(JSON.stringify(receipt, null, 2));
  if (!receipt.fresh || !discoveryRemoved) {
    throw new Error(`freshness matrix failed; evidence in ${outDir}`);
  }
  console.log(`matrix fresh; evidence in ${outDir}`);
}

await main();
