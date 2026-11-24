`longhorn-support-log` reports rotation degradation and JSON key collisions.
`RotatingLog::degraded_rotations()` counts rotations where a retained generation
could not shift, so retention loss is no longer hidden behind a clean write.
`RotatingLog::rejected_records()` counts JSON records dropped whole because two
keys became equal after redaction and bounding; such records are never written.
Both are additive; `LogStats` is unchanged. Contract 025 matches.
