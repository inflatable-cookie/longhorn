A swap that publishes the new bundle reports it as applied even if removing
the displaced one fails. The next apply reclaims the leftover using the
workspace receipt and a new publication marker. A bundle a failed rollback
retained is still never reclaimed automatically.
