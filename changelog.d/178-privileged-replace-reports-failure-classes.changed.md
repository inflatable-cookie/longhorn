`PrivilegedReplace::replace` returns `Result<(), InstallFailure>` instead of
`Result<(), String>`, so an escalated swap reports the contract's classes
rather than flattening every failure into `NotWritable`. A port that extracted
protected staging and could not complete both moves reports `Failed` with both
faults and the retained bundle's location; one whose own bounded extraction
refuses the artifact reports `MalformedArtifact`, which is terminal. Migration:
wrap the string you returned in `InstallFailure::NotWritable { detail }` when
escalation is declined or refused, and return `MalformedArtifact` or `Failed`
where the port knows better.
