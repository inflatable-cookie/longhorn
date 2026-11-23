import {
  parseJsonRecord,
  requireList,
  requireRecord,
  requireString,
} from "../../scripts/proof-json.ts";

export function requireNumber(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeError(`${label} must be a finite number`);
  }
  return value;
}

export function parseDiscovery(text: string) {
  const record = parseJsonRecord(text);
  return {
    appId: requireString(record.appId, "app ID"),
    pid: requireNumber(record.pid, "PID"),
    port: requireNumber(record.port, "port"),
    token: requireString(record.token, "discovery token"),
  };
}

export type SemanticNode = {
  elementRef: string;
  role: string;
  name?: string | undefined;
  children?: Array<SemanticNode> | undefined;
};

export function readSemanticNode(value: unknown): SemanticNode {
  const node = requireRecord(value, "semantic node");
  return {
    elementRef: requireString(node.elementRef, "element reference"),
    role: requireString(node.role, "node role"),
    name: node.name === undefined ? undefined : requireString(node.name, "node name"),
    children:
      node.children === undefined
        ? undefined
        : requireList(node.children, "child nodes").map(readSemanticNode),
  };
}

export function readToolResult(value: unknown) {
  const result = requireRecord(value, "MCP tool result");
  return {
    isError: result.isError === true,
    content: requireList(result.content, "MCP tool content").map((entry) =>
      requireRecord(entry, "MCP content entry"),
    ),
  };
}
