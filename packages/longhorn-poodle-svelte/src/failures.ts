/** The message a host shows for a thrown value, whatever it is. */
export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * For a failure with nowhere else to go: unmount cleanup and fire-and-forget
 * work whose outcome the session does not surface. Logging keeps it visible
 * to the operator's console without an unhandled rejection.
 */
export function reportFailure(error: unknown): void {
  console.error(error);
}
