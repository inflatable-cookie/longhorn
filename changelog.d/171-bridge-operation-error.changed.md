Catch `BridgeOperationError` from `@inflatable-cookie/longhorn/operation/bridge`
and branch on `outcome.kind`. Failed or cancelled terminals now throw
that error from `acceptTerminal()`. An indeterminate mutation carries the
host failure and must be reconciled before retrying.
