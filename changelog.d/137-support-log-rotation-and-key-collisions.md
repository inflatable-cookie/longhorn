`longhorn-support-log` reports rotation degradation and JSON key collisions.
`RotatingLog::degraded_rotations()` counts rotations where a step failed other
than on a missing path, so a failed shift is no longer reported as a plain clean
write. Zero means no counted rotation-step failure since the instance opened. `RotatingLog::rejected_records()`
counts JSON records dropped whole because two keys became equal after redaction
and bounding; such records are never written and are not part of `stats()`.
Both are additive; `LogStats` is unchanged. Contract 025 matches.
