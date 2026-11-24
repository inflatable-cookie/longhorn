Agent-control selection `begin` no longer panics without a Tokio runtime or
leaves a pending selection with no waiter. The TTL timer now starts on the
waiter's first poll, so publishing a selection needs no runtime, while awaiting
it still needs a runtime with timers enabled.
