export type ProofMember = { readonly name: string; readonly script: string };

export type MemberRun = { readonly exitCode: number; readonly stdout: string };

export type MemberTiming = {
  readonly member: string;
  readonly seconds: string;
  readonly result: "pass" | "fail";
  /** The member's captured stdout when it failed; empty when it passed. */
  readonly stdout: string;
};

/**
 * Runs every member in order and keeps going after a failure. A broken member
 * must not leave the later ones unverified: a milestone run reports every
 * failing proof in one pass. A member that cannot start counts as a failure.
 */
export async function runProofMembers(
  members: ReadonlyArray<ProofMember>,
  runMember: (member: ProofMember) => Promise<MemberRun>,
  clock: () => number = () => performance.now(),
): Promise<ReadonlyArray<MemberTiming>> {
  const timings: Array<MemberTiming> = [];
  for (const member of members) {
    const startedAt = clock();
    let run: MemberRun;
    try {
      run = await runMember(member);
    } catch (error) {
      run = { exitCode: 1, stdout: `failed to start proof ${member.name}: ${String(error)}` };
    }
    const passed = run.exitCode === 0;
    timings.push({
      member: member.name,
      seconds: ((clock() - startedAt) / 1000).toFixed(2),
      result: passed ? "pass" : "fail",
      stdout: passed ? "" : run.stdout,
    });
  }
  return timings;
}

export function failedMembers(timings: ReadonlyArray<MemberTiming>): ReadonlyArray<string> {
  return timings.flatMap((timing) => (timing.result === "fail" ? [timing.member] : []));
}

export function formatTimings(timings: ReadonlyArray<MemberTiming>): ReadonlyArray<string> {
  return [
    "proof:artifacts timings (seconds)",
    "member\tseconds\tresult",
    ...timings.map((timing) => `${timing.member}\t${timing.seconds}\t${timing.result}`),
  ];
}
