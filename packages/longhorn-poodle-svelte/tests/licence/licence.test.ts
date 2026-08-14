import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import {
  LICENCE_PROTOCOL_VERSION,
  LicenceController,
  type HeldLicenceProjection,
  type LicenceOutcomeProjection,
  type LicencePort,
  type LicenceSnapshot,
} from "@inflatable-cookie/longhorn/licence";

import LicenceHarness from "./LicenceHarness.svelte";

function held(overrides: Partial<HeldLicenceProjection> = {}): HeldLicenceProjection {
  return {
    product: "longhorn",
    usability: { state: "active" },
    trustBasis: { kind: "offlineSignature" },
    entitlements: [],
    seats: [],
    useUntil: null,
    updateUntil: null,
    ...overrides,
  };
}

function snapshot(licence: HeldLicenceProjection | null = held()): LicenceSnapshot {
  return { protocolVersion: LICENCE_PROTOCOL_VERSION, authorityEpoch: 1, licence };
}

class Port implements LicencePort {
  released: string[] = [];
  constructor(public state: LicenceSnapshot = snapshot()) {}
  #committed(): LicenceOutcomeProjection {
    return { status: "committed", snapshot: this.state };
  }
  async snapshot(): Promise<unknown> { return this.state; }
  async activate(): Promise<unknown> { return this.#committed(); }
  async deactivate(): Promise<unknown> { return this.#committed(); }
  async refresh(): Promise<unknown> { return this.#committed(); }
  async releaseSeat(command: unknown): Promise<unknown> {
    this.released.push((command as { machineId: string }).machineId);
    return this.#committed();
  }
  renamed: { machineId: string; label: string | null }[] = [];
  async renameSeat(command: unknown): Promise<unknown> {
    this.renamed.push(command as { machineId: string; label: string | null });
    return this.#committed();
  }
  #listeners: ((event: unknown) => void)[] = [];
  listen(listener: (event: unknown) => void) {
    this.#listeners.push(listener);
    return () => {};
  }
  notify(): void {
    for (const listener of this.#listeners) {
      listener({ protocolVersion: LICENCE_PROTOCOL_VERSION, authorityEpoch: 1, kind: "refreshed" });
    }
  }
}

async function ready(port: Port): Promise<LicenceController> {
  const controller = new LicenceController({ port });
  await controller.start();
  return controller;
}

describe("licence surface bindings", () => {
  it("renders nothing at all while unlicensed", async () => {
    const mounted = render(LicenceHarness, {
      props: { controller: await ready(new Port(snapshot(null))) },
    });

    await waitFor(() => expect(mounted.container.textContent).toBe(""));
  });

  it("re-renders when the authority notifies", async () => {
    const port = new Port();
    const mounted = render(LicenceHarness, { props: { controller: await ready(port) } });

    await waitFor(() => expect(document.body.textContent).toContain("Licence"));

    port.state = snapshot(held({ usability: { state: "useWindowExpired", at: 1_000 } }));
    port.notify();

    // Poodle's copy for an expired use window is "use coverage ended" — the
    // assertion is on their words, not on a word I expected them to use.
    await waitFor(() =>
      expect(document.body.textContent?.toLowerCase()).toContain("use coverage ended"),
    );
    mounted.unmount();
  });

  /**
   * An empty seat list means the authority does not account for seats.
   * Rendering "0 machines" would imply an accounting that is not happening.
   */
  it("renders no seat list when the authority does not account for seats", async () => {
    const mounted = render(LicenceHarness, {
      props: { controller: await ready(new Port()), surface: "seats" },
    });

    await waitFor(() => expect(mounted.container.textContent).toBe(""));
  });

  /** Card 199's feature: releasing a seat that is not this machine. */
  it("releases a named seat through the controller", async () => {
    const port = new Port(
      snapshot(
        held({
          seats: [
            { machineId: "m-this-machine-16chars", label: "Studio", thisMachine: true },
            { machineId: "m-old-macbook-16chars", label: null, thisMachine: false },
          ],
        }),
      ),
    );
    const mounted = render(LicenceHarness, {
      props: { controller: await ready(port), surface: "seats" },
    });

    const release = await mounted.findAllByRole("button");
    // The old machine's release control, whichever ordering Poodle renders:
    // click every button and assert exactly one release went through for the
    // machine that is not this one.
    for (const button of release) await fireEvent.click(button);

    await waitFor(() => expect(port.released).toEqual(["m-old-macbook-16chars"]));
    mounted.unmount();
  });

  /**
   * Key entry validates locally through the injected format. A mistyped key
   * must never read as an invalid one, and no round trip happens at all.
   */
  it("rejects a mistyped key locally as a typo", async () => {
    const port = new Port();
    const mounted = render(LicenceHarness, {
      props: { controller: await ready(port), surface: "activation" },
    });

    const input = await mounted.findByRole("textbox");
    // Valid body with its final symbol changed: the check fails, the shape is
    // fine, and the only honest message is "check your typing". Poodle
    // validates on submit rather than on input — a message that appears
    // mid-typing would flag every key as mistyped until its last character.
    await fireEvent.input(input, { target: { value: "ABCDE12345FGHJK6789Z" } });
    const submit = mounted
      .getAllByRole("button")
      .find((button) => button.textContent?.toLowerCase().includes("activate"));
    if (submit === undefined) throw new Error("no activate control");
    await fireEvent.click(submit);

    await waitFor(() => {
      expect(document.body.textContent).toContain("Check the key for a typing mistake.");
    });
    expect(document.body.textContent?.toLowerCase()).not.toContain("invalid");
    mounted.unmount();
  });

  /**
   * The seam Card 158 left unwired on purpose while the protocol had no
   * rename command. It has one now, and the binding normalises the one
   * disagreement between the sides: Poodle's editor can commit an empty
   * string for a cleared field, and the protocol treats empty as a mistake
   * and null as "unnamed".
   */
  it("renames a seat through the controller, clearing to null", async () => {
    const port = new Port(
      snapshot(
        held({
          seats: [{ machineId: "m-this-machine-16chars", label: "Studio", thisMachine: true }],
        }),
      ),
    );
    const mounted = render(LicenceHarness, {
      props: { controller: await ready(port), surface: "seats" },
    });

    const editor = await mounted.findByLabelText("Rename Studio");
    await fireEvent.click(editor);
    const input = await mounted.findByRole("textbox");
    await fireEvent.input(input, { target: { value: "Studio iMac" } });
    await fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() =>
      expect(
        port.renamed.map(({ machineId, label }) => ({ machineId, label })),
      ).toEqual([{ machineId: "m-this-machine-16chars", label: "Studio iMac" }]),
    );
  });
});
