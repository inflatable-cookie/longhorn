Bridge supervision now binds an observation to the service generation that
admitted it, so a readiness probe started against one service incarnation can no
longer report ready after a restart. `BridgeServiceMachine` and the TypeScript
`BridgeServiceRuntime` gain `begin` and `commit` beside the existing `execute`
and `observe`, and a delayed observation from a superseded generation is
rejected as stale instead of being committed.
