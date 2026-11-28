Agent-control page shim types are now generated from Rust. `SemanticNode.name`,
`SemanticNode.value` and `FileInputFile.mediaType` are required and nullable
(`string | null`) instead of optional: pass `mediaType: null` when there is
none, and compare absent names and values with `null`.
