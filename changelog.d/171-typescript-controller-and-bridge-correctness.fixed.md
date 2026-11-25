Update and licence controllers attempt all listener cleanup and discard stale
refreshes. Agent control matches DOM refs and label IDs literally. Bridge
validators use the generated domain ID rule and service failure codes.
Rust exposes the rule as `DomainId::MAX_BYTES`, `DomainId::SEPARATOR`,
`DomainId::SEGMENT_INITIAL`, and `DomainId::SEGMENT_ADDITIONAL`.
Notification mutations that settle after a stop leave the renderer empty.
