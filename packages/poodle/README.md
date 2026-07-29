# @longhorn/poodle

Private Svelte bindings from authoritative Longhorn layout state to public
Poodle `Tabs`, `DockRegion`, and `SplitView` components.

## Boundary

- Longhorn owns ids, registered policy, expected revisions, commands, and
  optimistic reconciliation.
- Poodle owns interaction semantics, markup, accessibility, and visual
  affordances.
- Consumers resolve labels, icons, and panel body snippets.
- The root package imports no Surface or transfer capability.

Create one binding per layout state:

```ts
import { createPoodleLayoutBinding } from "@longhorn/poodle";

const binding = createPoodleLayoutBinding({
  state: layoutState,
  definitions,
  nextRequestId,
  onError: reportError,
});
```

Pass the binding plus a container and region or sizing-slot id to the matching
component. Missing layout members and panel presentation are explicit errors.

`PoodleLayoutBinding.regionVisibilities()` adds projection-only compatible
region reveal. It never dispatches or changes the durable layout.

## Cross-window transfer

The optional `@longhorn/poodle/transfer` entry point binds `TransferState` to
Poodle's public `externalDragSource` and `externalDropTarget` props:

```ts
import {
  createPanelTransferDragSource,
  createPanelTransferDropTarget,
} from "@longhorn/poodle/transfer";
```

The source prepares on Poodle's pointer phase and writes the Longhorn payload
synchronously at dragstart. The target accepts only that checked payload and
commits through the authoritative transfer client. Consumers choose an
explicit leased zone or screen-point selector and clear transient reveal in
`onTerminal`.

The adapter references no Poodle-private selector, id, class, or MIME value.

## Preview Pin

Card 039 validates only Poodle artifact set
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`.
The package claims no broader Poodle compatibility before a published
prerelease exists.
