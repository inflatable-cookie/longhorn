`ConfigWindowPlacementSink` refuses a publication with `RequestFailed` while
four are still in flight. A caller that keeps timing out must wait for an
earlier flush to drain before its next request is accepted.
