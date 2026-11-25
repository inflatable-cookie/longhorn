A swap that publishes the new bundle but cannot remove the displaced one now
reports that fault instead of dropping it, and the next apply reclaims the
leftover through the workspace receipt and a new publication marker. A bundle
a failed rollback retained is still never reclaimed automatically.
