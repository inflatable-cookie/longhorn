Construct `HelperMessageKind::Progress` and
`IsolatedWindowRuntimeEventKind::Progress` phases with
`IsolatedWindowStartupPhase::new`. Protocol v1 still uses a JSON string and
now rejects phases above 128 UTF-8 bytes. Keep helper phase text within that
bound.
