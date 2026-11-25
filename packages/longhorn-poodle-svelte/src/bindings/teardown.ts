// Attempt every cleanup step and preserve the original error when only one fails.
export function throwTeardownFailures(results: ReadonlyArray<PromiseSettledResult<unknown>>): void {
  const failures: Array<unknown> = [];
  for (const result of results) {
    if (result.status === "rejected") {
      failures.push(result.reason);
    }
  }
  if (failures.length === 1) {
    // Preserve the host rejection, including serialized failures.
    throw failures[0];
  }
  if (failures.length > 1) {
    throw new AggregateError(failures, "Multiple cleanup steps failed");
  }
}
