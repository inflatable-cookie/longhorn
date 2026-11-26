A debounced layout presentation intent now applies to the value each flush
reads, so a structural publish between staging and flush no longer strands it.
`LayoutPresentationIntent::new` takes `(LayoutRequestId, LayoutMutationCommand)`
instead of a `LayoutMutationRequest`; the intent records no expected revision.
`requests()` returns `LayoutPresentationRequest` values. Migration: pass the
request id and the command, and drop the revision argument.
