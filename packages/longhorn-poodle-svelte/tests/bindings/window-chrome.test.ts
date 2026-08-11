import { describe, expect, it } from "vitest";

import {
  TRAFFIC_LIGHT_INSET_X,
  TRAFFIC_LIGHT_OPTICAL_OFFSET,
  trafficLightPosition,
} from "../../src/bindings/index.ts";

describe("trafficLightPosition", () => {
  // The convention this encodes was arrived at by hand in five applications
  // before anyone wrote it down. These are their real headers, so the rule
  // stays answerable to what consumers actually ship.
  it.each([
    { app: "loophole", header: 35.2, y: 19 },
    { app: "nucleus", header: 48, y: 25 },
    { app: "soundcheck", header: 48, y: 25 },
    { app: "figmatic", header: 62, y: 32 },
    { app: "bovine", header: 72, y: 37 },
  ])("places the group for $app's $header px header", ({ header, y }) => {
    expect(trafficLightPosition(header)).toEqual({ x: 18, y });
  });

  it("keeps the group hard left whatever the header", () => {
    for (const header of [24, 35.2, 48, 62, 72, 120]) {
      expect(trafficLightPosition(header).x).toBe(TRAFFIC_LIGHT_INSET_X);
    }
  });

  it("centres the group, then nudges it down by the optical offset", () => {
    // Stated separately from the table so a change to the offset shows up as a
    // deliberate edit here rather than as five mysterious numbers moving.
    for (const header of [40, 48, 64]) {
      expect(trafficLightPosition(header).y).toBe(
        header / 2 + TRAFFIC_LIGHT_OPTICAL_OFFSET,
      );
    }
  });

  it("rounds a fractional half to a whole pixel", () => {
    // 2.2rem is 35.2px, so the centre is 17.6 and the position must not be.
    expect(Number.isInteger(trafficLightPosition(35.2).y)).toBe(true);
  });

  it("refuses a height that cannot describe a titlebar", () => {
    for (const invalid of [0, -48, Number.NaN, Number.POSITIVE_INFINITY]) {
      expect(() => trafficLightPosition(invalid)).toThrow(RangeError);
    }
  });
});
