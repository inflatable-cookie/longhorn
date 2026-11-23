export interface TimerScheduler {
  // oxlint-disable-next-line quality/no-unknown-returns -- Each scheduler owns the handle it gives back to clear().
  set(delayMs: number, callback: () => void): unknown;
  clear(handle: unknown): void;
}

export const systemTimerScheduler: TimerScheduler = {
  set(delayMs, callback) {
    return globalThis.setTimeout(callback, delayMs);
  },
  clear(handle) {
    // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- systemTimerScheduler clears handles from its own setTimeout call.
    globalThis.clearTimeout(handle as ReturnType<typeof setTimeout>);
  },
};
