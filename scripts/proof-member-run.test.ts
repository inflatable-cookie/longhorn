import { expect, test } from "bun:test";
import type { ProofMember } from "./proof-member-run.ts";
import { failedMembers, formatTimings, runProofMembers } from "./proof-member-run.ts";

const members: ReadonlyArray<ProofMember> = [
  { name: "first", script: "scripts/first.ts" },
  { name: "second", script: "scripts/second.ts" },
  { name: "third", script: "scripts/third.ts" },
];

function ticking(): () => number {
  let now = 0;
  return () => {
    now += 1500;
    return now;
  };
}

test("a failing member does not stop the members after it", async () => {
  const ran: Array<string> = [];
  const timings = await runProofMembers(
    members,
    async (member) => {
      ran.push(member.name);
      return member.name === "first"
        ? { exitCode: 1, stdout: "first broke\n" }
        : { exitCode: 0, stdout: "" };
    },
    ticking(),
  );

  expect(ran).toEqual(["first", "second", "third"]);
  expect(timings.map((timing) => timing.result)).toEqual(["fail", "pass", "pass"]);
  expect(failedMembers(timings)).toEqual(["first"]);
  expect(timings[0]?.stdout).toBe("first broke\n");
  expect(timings[1]?.stdout).toBe("");
});

test("a member that cannot start is a failure and the run continues", async () => {
  const timings = await runProofMembers(members, async (member) => {
    if (member.name === "second") {
      throw new Error("spawn refused");
    }
    return { exitCode: 0, stdout: "" };
  });

  expect(timings.map((timing) => timing.member)).toEqual(["first", "second", "third"]);
  expect(failedMembers(timings)).toEqual(["second"]);
  expect(timings[1]?.stdout).toContain("failed to start proof second: Error: spawn refused");
});

test("timings are seconds from the clock and the table names every member", async () => {
  const timings = await runProofMembers(
    members.slice(0, 1),
    async () => ({ exitCode: 0, stdout: "" }),
    ticking(),
  );

  expect(timings[0]?.seconds).toBe("1.50");
  expect(formatTimings(timings)).toEqual([
    "proof:artifacts timings (seconds)",
    "member\tseconds\tresult",
    "first\t1.50\tpass",
  ]);
});
