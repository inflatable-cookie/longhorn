`ActivationRequest` no longer derives `Debug`. It prints the URL, the body
length, and the header names; body bytes and header values are never
formatted, so `{:?}` on a request or on the enclosing `Activation::Exchange`
cannot carry a licence key or account token out. Additive; public types and
fields are unchanged.
