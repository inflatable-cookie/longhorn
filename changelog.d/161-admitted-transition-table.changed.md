The connection state-to-state transition table is now declared once,
`BridgeConnectionState::ADMITTED_TRANSITIONS`, and generated as
`BRIDGE_ADMITTED_CONNECTION_TRANSITIONS`. The TypeScript runtime reads that
table in place of its hand-written copy.
