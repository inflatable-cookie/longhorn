Match on `TransferHostOperation` and `AgentControlPathOperation` when reading
the public operation fields on `TransferRuntimeError::HostCall` and
`AgentControlMountError::Path`. These fields now carry typed enums instead of
string labels; match on enum variants rather than comparing strings, and use a
wildcard arm because both enums are non-exhaustive.
