Job progress and terminal listeners now check the request and job identifiers
before they decode the payload. Another job's event on the shared Tauri channel
is dropped without failing the current job or tearing down its listeners, and
input after a job's terminal is no longer decoded. A malformed event for the
job itself still ends the job, as before.
`BridgeJobTracker` gains `correlates`, and `parseBridgeJobCorrelation` reads
only the identifiers, both beside the existing exports.
