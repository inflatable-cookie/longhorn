Match on `TransferHostOperation` and `AgentControlPathOperation` when reading
the public operation fields on `TransferRuntimeError::HostCall` and
`AgentControlMountError::Path`. These fields now carry typed enums instead of
string labels; use enum variants in matches rather than comparing strings.
