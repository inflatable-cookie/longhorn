Explicit flushes also refuse new publications once four are still in flight,
returning `RequestFailed` instead of starting another worker. A caller that keeps
timing out can no longer accumulate blocked workers.
