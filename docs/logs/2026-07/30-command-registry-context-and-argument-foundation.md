# Command Registry, Context, And Argument Foundation

Date: 2026-07-30
Card: 056
Roadmap: g01.010

## Result

Added `longhorn-command`, a pure Rust command declaration, context, argument,
discovery, and search library. The crate seals immutable registry generations
without current product state, execution, configuration, transport, or UI.

## Registry

- distinct bounded command, context, category, keyword, route, capability,
  field, and enum-value types
- one finite parent tree with a parentless `global` root
- registered capability references independent of runtime availability
- explicit palette, menu, shortcut, settings, help, and hidden visibility
- explicit editable-text admission policy
- consumer-owned opaque routes that are not bridge operation names
- bounded labels, descriptions, icons, keywords, lists, graph depth, fields,
  enums, strings, and search queries
- canonical command/context/capability order and SHA-256 content digest
- generation excluded from content identity

## Arguments

V1 accepts no arguments as JSON `null`, or one declared object. Object fields
are boolean, finite number, bounded integer, bounded string, or closed enum.
Validation inserts declared defaults and returns one `BTreeMap`-ordered value.

Unknown fields, missing required fields, objects in fields, arrays, wrong
primitive types, out-of-range values, oversized strings, unknown enums,
non-finite numbers, invalid bounds, empty object schemas, duplicate fields or
enum values, and invalid defaults fail with stable structural categories.
Product semantic validation remains downstream.

## Discovery

One sealed command identity projects to every discovery surface. Search
requires every query term and ranks label, keyword, category, command id, then
description. Score, lowercase label, and command id provide stable ordering.
Hidden commands never enter a surface projection.

## Fixture Matrix

| Shape | Contexts | Commands | Proof |
| --- | --- | --- | --- |
| Loophole | global/project/surface/editor/region/panel | transport, editor, panel | rich hierarchy and arguments need no DAW types |
| Jetstream | global/editor | file and format | minimal composition needs no Surface or bridge |

Additional fixtures cover duplicate and malformed ids, missing references,
cycles, multiple roots, depth, contradictory visibility, limits, strict
serialization, insertion order, generation-independent digest, ranking, and
surface filtering.

## Boundary Audit

- normal dependencies: `longhorn-core`, Serde, `serde_json`, SHA-256
- no config, settings, bridge, Tauri, async runtime, renderer, Svelte, Poodle,
  or donor dependency
- registry identity does not depend on focus, selection, connection, or
  availability
- command ids remain semantic; routes remain consumer-owned
- raw JSON exists only at the structural validation boundary and cannot survive
  as a normalized nested or arbitrary value

## Validation

- `effigy test:command-core`
- 18 command contract tests plus 30 core tests
- `cargo clippy -p longhorn-command --all-targets -- -D warnings`
- `effigy fmt:rust`
- `effigy qa:northstar:g01-command-core`
- `cargo doc -p longhorn-command --no-deps`

## Next

Card 057 is ready. Add fresh availability and injected execution admission
without granting renderer, bridge, or product authority to the registry.
