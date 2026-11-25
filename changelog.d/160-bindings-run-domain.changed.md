Per-domain `<domain>::run(mode)` functions are replaced by
`<domain>::render() -> Result<Vec<Artifact>, Box<dyn Error>>` and
`longhorn_bindings::run_domain(domain, mode)`; migration: call
`longhorn_bindings::run_domain(domain, mode)` (or `"all"`).
