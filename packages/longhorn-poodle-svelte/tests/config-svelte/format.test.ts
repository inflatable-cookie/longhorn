import { describe, expect, it } from "vitest";

import { formatArchiveTimestamp } from "../../src/config/poodle/format.ts";

describe("formatArchiveTimestamp", () => {
  it("shows an ISO timestamp in the requested locale rather than as received", () => {
    const shown = formatArchiveTimestamp("2026-11-22T09:30:00Z", "en-GB");
    expect(shown).toContain("2026");
    expect(shown).not.toContain("T09");
  });

  it("shows a value that does not parse as received", () => {
    expect(formatArchiveTimestamp("not a date")).toBe("not a date");
  });
});
