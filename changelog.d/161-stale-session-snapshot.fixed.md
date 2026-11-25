An older same-session snapshot no longer replaces a newer accepted baseline. The
stream tracker refuses it as `staleSnapshot`, whenever it resolves, and keeps the
baseline. A stale resync reply is dropped: the TypeScript stream keeps its current
snapshot and stays live, where it previously moved the baseline back and could
apply an already-applied event a second time.
