import { expect, test } from "bun:test";
import { assertProductNeutral } from "./artifacts.ts";

test("allows prose and longer identifiers that mention network, fixture and credential words", () => {
  const source = [
    "// Subscribe to the event source and reuse the web socket's connection.",
    "// We fetch the data from a local fixture.",
    "const eventSourceId = currentSubscription();",
    "const $EventSource = bridgeName;",
    "const api_key_present = false;",
    "prefetch (page);",
  ].join("\n");
  expect(() => assertProductNeutral("test artifact", source)).not.toThrow();
});

test.each([
  ["const reader: EventSource = source", "eventsource"],
  ["new EventSource(url)", "eventsource"],
  ["import { read } from 'eventsource'", "eventsource"],
  ["new WebSocket (url)", "websocket"],
  ["client.fetch \n (url)", "fetch("],
  ["use reqwest::Client;", "reqwest"],
  ["tokio-tungstenite = '*'", "tokio-tungstenite"],
  ["const token = credentials.access_token", "access_token"],
  ["const name = 'ordered-streams';", "ordered-streams"],
  ["const domain = 'fixture.catalog';", "fixture."],
])("rejects %s as the %s marker", (source, marker) => {
  expect(() => assertProductNeutral("test artifact", source)).toThrow(
    `test artifact contains forbidden marker: ${marker}`,
  );
});
