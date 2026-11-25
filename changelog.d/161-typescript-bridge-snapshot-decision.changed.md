The TypeScript `BridgeSnapshotDecision` type gains `"staleSnapshot"`. Migration:
handle it beside `"supersededSession"` and `"wrongDomain"`. The stream does not
throw on it.
