Remove the unscoped update progress path: `listenProgress` is gone from
`UpdatePort`, `CheckedUpdatePort`, `UpdateClient` and `SerializedUpdatePort`.
A custom port that reports live bytes must implement `prepareWithProgress`,
which delivers one prepare invocation's reports.
