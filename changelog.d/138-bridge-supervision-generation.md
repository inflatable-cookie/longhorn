Bridge supervision now binds each observation to the service incarnation that
admitted it. A readiness probe can no longer report ready for a service that was
restarted in the meantime, while overlapping readiness probes of one attempt and
a shutdown admitted beside a probe keep their previous outcomes.
`BridgeServiceMachine` and the TypeScript `BridgeServiceRuntime` gain `begin`
and `commit` beside the existing `execute` and `observe`, and an observation from
a superseded incarnation is rejected as stale instead of being committed.
