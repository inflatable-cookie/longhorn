Keep renderer subscriptions active when a consumer callback throws. Report
malformed events separately, release every job listener on teardown, and
preserve the original rejection from a failed port operation.
