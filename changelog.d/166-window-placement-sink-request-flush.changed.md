`ConfigWindowPlacementSink::request_flush` no longer writes on the caller's
thread. A caller that drops the ticket gets no synchronous write. Migration:
wait for the flush through `flush_window_placements`, or on the ticket.
