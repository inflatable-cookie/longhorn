import { describe, expect, test } from "bun:test";

import {
  BridgeConnectionRuntime,
  BridgeQueryRetryRuntime,
  BridgeRuntimeError,
  type BridgeRuntimeBackoff,
  type BridgeRuntimeClock,
} from "@longhorn/bridge";

import { fixture } from "./support.ts";

class Clock implements BridgeRuntimeClock {
  value = 100;

  now(): number {
    return this.value;
  }
}

const backoff: BridgeRuntimeBackoff = {
  delay: (_retryClass, attempt) => attempt * 25,
};

describe("checked bridge connection runtime", () => {
  test("becomes ready only after negotiation and required authority", () => {
    const clock = new Clock();
    const runtime = new BridgeConnectionRuntime(clock, backoff, 2);
    expect(runtime.connect().current.state).toBe("connecting");
    expect(runtime.transportReady().current.state).toBe("negotiating");

    expect(() =>
      runtime.acceptNegotiation(fixture.negotiation, [{
        domainId: "missing.domain",
        authority: "writable",
      }])
    ).toThrow(BridgeRuntimeError);
    expect(runtime.status.state).toBe("negotiating");
    expect(runtime.session).toBeUndefined();

    const ready = runtime.acceptNegotiation(fixture.negotiation, [{
      domainId: "example.workspace",
      authority: "writable",
    }]);
    expect(ready.current.state).toBe("ready");
    expect(ready.sessionId).toBe("session:fixture");
  });

  test("reconnect invalidates old session and authority evidence", () => {
    const clock = new Clock();
    const runtime = readyRuntime(clock);
    const current = {
      sessionId: "session:fixture",
      domainId: "example.workspace",
      authorityEpoch: 3,
      sequence: 1,
    };
    expect(runtime.classifyCursor(current)).toBe("current");

    const reconnect = runtime.reconnect("afterReconnect");
    expect(reconnect.current.state).toBe("reconnecting");
    expect(reconnect.reconnect).toEqual({
      attempt: 1,
      retryClass: "afterReconnect",
      notBefore: 125,
    });
    expect(runtime.classifyCursor(current)).toBe("supersededSession");

    clock.value = 125;
    runtime.transportReady();
    const replacement = structuredClone(fixture.negotiation) as any;
    replacement.sessionId = "session:replacement";
    replacement.domainAuthorities[0].authorityEpoch = 4;
    runtime.acceptNegotiation(replacement, []);
    expect(runtime.classifyCursor(current)).toBe("supersededSession");
    expect(runtime.classifyCursor({
      ...current,
      sessionId: "session:replacement",
    })).toBe("staleAuthority");
  });

  test("bounded reconnect exhausts to offline", () => {
    const clock = new Clock();
    const runtime = readyRuntime(clock, 1);
    runtime.reconnect("afterReconnect");
    expect(() => runtime.transportReady()).toThrow(BridgeRuntimeError);
    clock.value = 125;
    runtime.transportReady();
    expect(runtime.reconnect("afterReconnect").current.state).toBe("offline");
    expect(runtime.close().current.state).toBe("closed");
  });

  test("never retry class goes offline without invoking backoff", () => {
    const clock = new Clock();
    const runtime = readyRuntime(clock);
    expect(runtime.reconnect("never").current.state).toBe("offline");
  });

  test("degrade mismatch unauthorized failure and shutdown are explicit", () => {
    const clock = new Clock();
    const degraded = readyRuntime(clock, 0);
    expect(degraded.degrade("capabilityChanged").current.state).toBe(
      "degraded",
    );

    const incompatible = negotiatingRuntime(clock);
    expect(incompatible.incompatible().current.state).toBe("incompatible");
    expect(incompatible.close().current.state).toBe("closed");

    const unauthorized = negotiatingRuntime(clock);
    expect(unauthorized.unauthorized().current.state).toBe("unauthorized");

    const failed = new BridgeConnectionRuntime(clock, backoff, 0);
    failed.connect();
    expect(failed.fail().current.state).toBe("failed");
  });
});

test("query retry scheduling is bounded and injected", () => {
  const clock = new Clock();
  const runtime = new BridgeQueryRetryRuntime(clock, backoff, 2);
  expect(runtime.schedule("retry", "afterBackoff")?.notBefore).toBe(125);
  expect(runtime.schedule("retry", "afterBackoff")?.notBefore).toBe(150);
  expect(runtime.schedule("retry", "afterBackoff")).toBeUndefined();
  runtime.reset();
  expect(runtime.schedule("retry", "afterBackoff")?.attempt).toBe(1);
  expect(runtime.schedule("retry", "never")).toBeUndefined();
  expect(runtime.schedule("doNotRetry", "afterBackoff")).toBeUndefined();
  expect(() => runtime.schedule("retry", "future" as any)).toThrow(
    BridgeRuntimeError,
  );
});

function readyRuntime(
  clock: Clock,
  reconnectLimit = 2,
): BridgeConnectionRuntime {
  const runtime = new BridgeConnectionRuntime(
    clock,
    backoff,
    reconnectLimit,
  );
  runtime.connect();
  runtime.transportReady();
  runtime.acceptNegotiation(fixture.negotiation, []);
  return runtime;
}

function negotiatingRuntime(clock: Clock): BridgeConnectionRuntime {
  const runtime = new BridgeConnectionRuntime(clock, backoff, 0);
  runtime.connect();
  runtime.transportReady();
  return runtime;
}
