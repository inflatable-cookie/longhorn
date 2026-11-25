PTY sessions on macOS recover from transient master allocation races.
Persistent failures still report the raw OS error. A completed PTY input
write succeeds when the child exits immediately after accepting the input.
