# longhorn-native-content-isolated-window

Generic process-isolated native-content coordination. The crate executes
`isolated_window` plans through an injected owner runtime, admits only current
generation events, bounds helper correlation and request queues, and reports
cooperative close, timeout, helper loss, or owner termination exactly.

The strict shared helper protocol carries exact version `1` and content-area
size operations only. It contains no outer position, raw handle, plugin,
authorization, renderer, Signal, audio, or MIDI payload.

Consumers provide process launch, native content creation, authorization,
transport I/O, and safe owner termination. macOS is proved by the packaged
fixture. Windows and Linux are unsupported for this mechanism.
