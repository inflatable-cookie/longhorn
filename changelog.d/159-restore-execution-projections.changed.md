Restore execution failures and adapter and recovery receipts now take
`RestoreExecutionStageProjection`, `RestoreFailureTerminalProjection`,
`RestoreAdapterOutcomeProjection`, and `RestoreRecoveryResultProjection`
enum values in Rust. Use the matching variant in constructors and matches.
TypeScript uses the corresponding literal unions. Existing documented JSON
strings remain valid. Narrow arbitrary strings before assigning these fields.
