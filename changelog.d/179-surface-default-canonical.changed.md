`RegisteredSurfaceDomain::new` refuses a default that is not in canonical
structural order, with `RegisteredSurfaceDomainError::InvalidDefault` and code
`NonCanonicalDocument`, instead of re-sorting it silently. Migration: sort the
compiled default into canonical order before registration, for example with
`longhorn_surfaces::normalize_document` in a consumer test.
