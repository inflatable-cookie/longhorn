Fixed `longhorn-bindings` stale-artifact scanning, drift recovery hints,
domain boilerplate, and `field_map` return shape: `surfaces` and
`surface-transfer` now pass their canonical directory slugs (`"surfaces"` and
`"surface-transfer"`) instead of display labels, and `generation::apply`
validates that every artifact path stays under its domain's `generated/` or
`fixtures/` directory.
