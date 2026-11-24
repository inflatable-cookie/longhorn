Job progress and terminal listeners now check the request and job identifiers
before they decode the payload. Another job's event on the shared Tauri channel
is dropped without failing the current job or tearing down its listeners, even
when one of its identifiers is unreadable, and input after a job's terminal is
no longer decoded. A malformed event under the job's own request still ends the
job, as before. `BridgeJobTracker` gains `admits`, beside the existing exports.
