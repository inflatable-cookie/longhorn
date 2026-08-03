# Layout Ratio Serde And Schema Caps

Date: 2026-08-03
Card: 138
Roadmap: g02.001

## Result

`LayoutRatio` now validates on deserialization, mirroring `ScaleFactor`. The
derived transparent `Deserialize` was the only invariant bypass: the field is
private and every other constructor caps at `RATIO_ONE_MILLIONTHS`. With
serde closed, the ≤1.0 invariant is total by construction, so sizing schema
bounds cannot exceed 100% through any path; no separate `validate_schema` cap
is reachable or added.

## Exact Evidence

- `serde_json::from_str::<LayoutRatio>("1000001")` fails typed; `"1000000"`
  passes
- schema definition with slot `maximum` 3_000_000 fails deserialization
- document with sizing ratio 1_000_001 fails deserialization
- `SetSizingSlot` command with ratio 1_000_001 fails deserialization
- layout suite 38 pass; layout TS suite 16 pass, 151 expects
- `check:layout-bindings` current, no regeneration needed; wire shape
  unchanged for valid input
- `cargo check --workspace`, Clippy, and `check:layout-package` pass

## Deviation From Card

The card planned a `validate_schema` maximum cap. Recorded instead as
enforced by construction: with the validating `Deserialize`, no
`LayoutRatio > ONE` can exist, so the schema check would be dead code. The
three serde-entry rejection tests carry the guarantee.
