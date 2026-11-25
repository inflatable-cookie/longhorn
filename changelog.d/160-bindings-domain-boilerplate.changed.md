Replaced duplicated `RenderedProtocol` / `run()` boilerplate across domain
modules with `generation::DomainArtifacts` + `build_domain_artifacts`, a
single `DOMAINS` table in `longhorn_bindings`, Serde `enum_names_json` for
unit enums across all 15 domains, and `effigy check:bindings` /
`generate:bindings` calling `longhorn-bindings all <check|write>`.
