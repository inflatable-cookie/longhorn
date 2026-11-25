`take_requests` now returns `AdmittedIsolatedContentRequest` values. Inspect
the helper payload with `request()` and pass the admitted value to
`decide_resize`, which checks the revision recorded when the adapter admitted
the request.
