Explicit window-placement flushes are now bounded. `ConfigWindowPlacementSink`
starts each publication on a worker and returns its acknowledgement channel, so
`flush_window_placements` reports `TimedOut` when a publication outlasts the
request's timeout. A publication that times out may still complete afterwards,
and its placements stay staged until one succeeds.
