Fixed `longhorn-bindings` stale-artifact scanning, drift recovery hints,
domain boilerplate, and `field_map` return shape:

- `surfaces` and `surface-transfer` now pass their canonical directory slugs
  (`"surfaces"` and `"surface-transfer"`) instead of display labels, and
  `generation::apply` validates that every artifact path stays under its
  domain's `generated/` or `fixtures/` directory.
- Drift failures and `@generated` file headers now point to
  `effigy generate:bindings` (`generation::GENERATE_TASK`) instead of
  nonexistent per-domain `generate:<domain>` tasks.
- Replaced duplicated `RenderedProtocol` / `run()` boilerplate across domain
  modules with `generation::DomainArtifacts` + `build_domain_artifacts`, a
  single `DOMAINS` table in `longhorn_bindings`, Serde `enum_names_json` for
  unit enums across all 15 domains, and `effigy check:bindings` /
  `generate:bindings` calling `longhorn-bindings all <check|write>`.

### Breaking

- `longhorn_bindings::generation::apply(domain, mode, artifacts)` no longer
  takes `generate_task: &str`; migration: remove the task argument (the crate
  always stamps `generate:bindings`).
- `longhorn_bindings::generation::tagged_variants(declaration)` no longer takes
  `tag: &str` and detects the discriminant via `detect_discriminant`;
  migration: drop the tag argument.
- `longhorn_bindings::generation::field_map(task, constant, declarations)` now
  returns `String` instead of `(String, Vec<String>)`; migration: bind the
  returned string directly instead of destructuring and discarding skipped
  union names.
- Per-domain `<domain>::run(mode)` functions are replaced by
  `<domain>::render() -> Result<Vec<Artifact>, Box<dyn Error>>` and
  `longhorn_bindings::run_domain(domain, mode)`; migration: call
  `longhorn_bindings::run_domain(domain, mode)` (or `"all"`).
