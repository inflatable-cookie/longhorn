import { Window } from "happy-dom";

import {
  installAgentControlShim,
  type AgentControlApi,
  type SemanticNode,
  type ShimWorld,
} from "../../src/agent-control/index.ts";

export function openPage(html: string, url = "https://app.example/test"): Window {
  const window = new Window({ url });
  window.document.write(
    `<!doctype html><html><head><title>Proof</title></head><body>${html}</body></html>`,
  );
  return window;
}

export function install(window: Window): AgentControlApi {
  return installAgentControlShim(window as unknown as ShimWorld);
}

export function findByName(node: SemanticNode, name: string): SemanticNode | undefined {
  if (node.name === name) return node;
  for (const child of node.children) {
    const match = findByName(child, name);
    if (match) return match;
  }
  return undefined;
}

export function findByRole(node: SemanticNode, role: string): SemanticNode | undefined {
  if (node.role === role) return node;
  for (const child of node.children) {
    const match = findByRole(child, role);
    if (match) return match;
  }
  return undefined;
}

export function collectRoles(node: SemanticNode): string[] {
  return [node.role, ...node.children.flatMap(collectRoles)];
}
