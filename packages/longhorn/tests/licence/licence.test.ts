import { describe, expect, test } from "bun:test";
import {
  LICENCE_PROTOCOL_VERSION,
  LicenceClient,
  LicenceController,
  LicenceValidationError,
  SerializedLicencePort,
  assertLicenceOutcome,
  assertLicenceSnapshot,
  createDirectLicencePort,
  type HeldLicenceProjection,
  type LicenceOutcomeProjection,
  type LicencePort,
  type LicenceSnapshot,
  type LicenceUsabilityProjection,
} from "../../src/licence/index.ts";

function held(overrides: Partial<HeldLicenceProjection> = {}): HeldLicenceProjection {
  return {
    product: "longhorn",
    usability: { state: "active" },
    trustBasis: { kind: "offlineSignature" },
    entitlements: [{ id: "pro", atMost: null }],
    useUntil: null,
    updateUntil: null,
    ...overrides,
  };
}

function snapshot(licence: HeldLicenceProjection | null = held()): LicenceSnapshot {
  return {
    protocolVersion: LICENCE_PROTOCOL_VERSION,
    authorityEpoch: 2,
    licence,
  };
}

function committed(value: LicenceSnapshot = snapshot()): LicenceOutcomeProjection {
  return { status: "committed", snapshot: value };
}

class Port implements LicencePort {
  constructor(
    private state: LicenceSnapshot = snapshot(),
    private outcome: () => LicenceOutcomeProjection = () => committed(this.state),
  ) {}
  async snapshot(): Promise<unknown> { return this.state; }
  async activate(): Promise<unknown> { return this.outcome(); }
  async deactivate(): Promise<unknown> { return this.outcome(); }
  async refresh(): Promise<unknown> { return this.outcome(); }
  async listen(): Promise<() => void> { return () => {}; }
}

describe("licence validation", () => {
  test("a held and an unlicensed snapshot both pass", () => {
    expect(() => assertLicenceSnapshot(snapshot())).not.toThrow();
    expect(() => assertLicenceSnapshot(snapshot(null))).not.toThrow();
  });

  test("an unknown key is rejected", () => {
    expect(() => assertLicenceSnapshot({ ...snapshot(), extra: 1 })).toThrow(
      LicenceValidationError,
    );
  });

  test("a surplus key on a usability variant is rejected", () => {
    const usability = { state: "active", at: 1 } as unknown as LicenceUsabilityProjection;
    expect(() => assertLicenceSnapshot(snapshot(held({ usability })))).toThrow(
      LicenceValidationError,
    );
  });

  /**
   * The rule the whole domain rests on. A projection that carried a token,
   * signature or key id must fail at the boundary rather than reach a surface
   * that might display it.
   */
  test("credential material anywhere in a projection is refused", () => {
    for (const key of ["token", "signature", "keyId", "credential", "clientSecret"]) {
      const poisoned = { ...snapshot(), licence: { ...held(), [key]: "x" } };
      expect(() => assertLicenceSnapshot(poisoned)).toThrow(LicenceValidationError);
    }
  });

  /**
   * `clockRefused` carries no timestamp, and must not be given one. A licence
   * refused because the machine clock moved is not expired.
   */
  test("clock refused is distinct from every expiry state", () => {
    expect(() =>
      assertLicenceSnapshot(snapshot(held({ usability: { state: "clockRefused" } }))),
    ).not.toThrow();
    const dated = { state: "clockRefused", at: 10 } as unknown as LicenceUsabilityProjection;
    expect(() => assertLicenceSnapshot(snapshot(held({ usability: dated })))).toThrow(
      LicenceValidationError,
    );
  });

  /**
   * `atMost` is `u64` in Rust and was binding as `bigint`, which `JSON.parse`
   * never produces — the type described a wire that could not occur. It is
   * annotated as `number | null` now, and this is what would catch a
   * regression.
   */
  test("an entitlement bound is a plain number or absent", () => {
    expect(() =>
      assertLicenceSnapshot(snapshot(held({ entitlements: [{ id: "seats", atMost: 5 }] }))),
    ).not.toThrow();
    expect(() =>
      assertLicenceSnapshot(snapshot(held({ entitlements: [{ id: "pro", atMost: null }] }))),
    ).not.toThrow();
  });

  test("a rejection carries a known code and the state as it remains", () => {
    const outcome: LicenceOutcomeProjection = {
      status: "rejected",
      code: "noSeatsFree",
      snapshot: snapshot(),
    };
    expect(() => assertLicenceOutcome(outcome)).not.toThrow();
    expect(() => assertLicenceOutcome({ ...outcome, code: "invented" })).toThrow(
      LicenceValidationError,
    );
  });
});

describe("licence client", () => {
  test("an outbound credential is validated before it is sent", async () => {
    const client = new LicenceClient(new Port());

    await expect(
      client.activate({
        protocolVersion: LICENCE_PROTOCOL_VERSION,
        authorityEpoch: 2,
        credential: { kind: "key", key: "" },
      }),
    ).rejects.toThrow(LicenceValidationError);
  });

  test("a malformed reply is refused rather than returned", async () => {
    const port = createDirectLicencePort({
      snapshot: async () => ({ nonsense: true }),
      activate: async () => committed(),
      deactivate: async () => committed(),
      refresh: async () => committed(),
    });

    await expect(new LicenceClient(port).snapshot()).rejects.toThrow(LicenceValidationError);
  });

  test("the serialized port survives a structured-clone round trip", async () => {
    await expect(new LicenceClient(new SerializedLicencePort(new Port())).snapshot()).resolves.toEqual(
      snapshot(),
    );
  });
});

describe("licence controller", () => {
  async function ready(state: LicenceSnapshot): Promise<LicenceController> {
    const controller = new LicenceController({ port: new Port(state) });
    await controller.start();
    return controller;
  }

  test("start reads the snapshot and reports ready", async () => {
    const controller = await ready(snapshot());

    expect(controller.status).toEqual({ kind: "ready" });
    expect(controller.activated).toBeTrue();
    expect(controller.usable).toBeTrue();
  });

  test("unlicensed is the absence of a licence, not a usability state", async () => {
    const controller = await ready(snapshot(null));

    expect(controller.activated).toBeFalse();
    expect(controller.usability).toBeUndefined();
    expect(controller.attention).toBe("actionable");
  });

  /**
   * Card 158 step 6. A renewal that has not yet succeeded, inside its lease,
   * is a backend outage rather than the customer's problem — and raising it
   * turns an outage into a support ticket from someone who has already paid.
   */
  test("an in-lease renewal failure asks for no attention", async () => {
    const controller = await ready(
      snapshot(held({ usability: { state: "inGrace", until: 5_000 } })),
    );

    expect(controller.usable).toBeTrue();
    expect(controller.attention).toBe("none");
  });

  test("an expired use window is actionable", async () => {
    const controller = await ready(
      snapshot(held({ usability: { state: "useWindowExpired", at: 5_000 } })),
    );

    expect(controller.usable).toBeFalse();
    expect(controller.attention).toBe("actionable");
  });

  /**
   * Card 158 step 5. "Your subscription lapsed" and "your updates lapsed but
   * the app keeps working" are different sentences, so they are different
   * reads. A perpetual licence past its update window is informational, never
   * an error.
   */
  test("the two windows are separate reads", async () => {
    const controller = await ready(snapshot(held({ useUntil: 9_000, updateUntil: 1_000 })));

    expect(controller.useUntil).toBe(9_000);
    expect(controller.updateUntil).toBe(1_000);
    expect(controller.usable).toBeTrue();
    expect(controller.attention).toBe("informational");
  });

  test("entitlements are opaque ids with optional bounds", async () => {
    const controller = await ready(
      snapshot(held({ entitlements: [{ id: "pro", atMost: null }, { id: "seats", atMost: 5 }] })),
    );

    expect(controller.entitlements).toEqual(["pro", "seats"]);
    expect(controller.holds("pro")).toBeTrue();
    expect(controller.holds("absent")).toBeFalse();
    expect(controller.limit("seats")).toBe(5);
    expect(controller.limit("pro")).toBeUndefined();
  });

  test("a rejection is recorded with its code and cleared by the next commit", async () => {
    let refuse = true;
    const controller = new LicenceController({
      port: new Port(snapshot(), () =>
        refuse ? { status: "rejected", code: "revoked", snapshot: snapshot() } : committed(),
      ),
    });
    await controller.start();

    await controller.deactivate();
    expect(controller.lastRejection).toBe("revoked");

    refuse = false;
    await controller.refreshLease();
    expect(controller.lastRejection).toBeUndefined();
  });

  test("a command before the first read fails saying so", async () => {
    const controller = new LicenceController({ port: new Port() });

    await controller.deactivate();

    expect(controller.status.kind).toBe("failed");
    expect(String((controller.status as { error: unknown }).error)).toContain("has not been read");
  });
});
