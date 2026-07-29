export interface TimerScheduler {
  set(delayMs: number, callback: () => void): unknown;
  clear(handle: unknown): void;
}

export const systemTimerScheduler: TimerScheduler = {
  set(delayMs, callback) {
    return globalThis.setTimeout(callback, delayMs);
  },
  clear(handle) {
    globalThis.clearTimeout(handle as ReturnType<typeof setTimeout>);
  },
};
