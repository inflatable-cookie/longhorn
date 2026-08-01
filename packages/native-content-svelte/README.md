# @longhorn/native-content-svelte

Per-mounted-instance Svelte coordination for `@longhorn/native-content`.

The consumer creates one `NativeContentSession` per native island, binds the
exact public viewport element with `use:nativeContentViewport`, and mounts the
session with `useNativeContentSession`. The session reads host window,
generation, rounding, and presence from checked authority. The consumer must
provide the current scale in integer thousandths plus final visibility, focus,
and input-routing policy.

```svelte
<script lang="ts">
  import { Surface } from "@poodle/svelte";
  import {
    NativeContentSession,
    nativeContentViewport,
    resolveNativeContentVisibility,
    useNativeContentSession,
  } from "@longhorn/native-content-svelte";

  let { client, scale, obscured } = $props();
  const session = new NativeContentSession({
    client,
    scale,
    visibility: resolveNativeContentVisibility([
      { reason: "consumer_overlay", active: obscured },
    ]),
    focus: "unchanged",
    inputRouting: "native_direct",
  });
  $effect(() => {
    session.setScale(scale);
    session.setVisibilityPolicy(resolveNativeContentVisibility([
      { reason: "consumer_overlay", active: obscured },
    ]));
  });
  useNativeContentSession(session);
</script>

<Surface label="Browser">
  <div class="product-browser-viewport" use:nativeContentViewport={session}></div>
</Surface>
```

Poodle remains outside the package graph. Longhorn binds only the element the
consumer supplies; it does not query Poodle markup, discover overlays, infer
occlusion, read `devicePixelRatio`, or handle semantic input. Consumer state
changes call `setScale`, `setVisibilityPolicy`, `setFocusIntent`, or
`setInputRouting`.

Every start creates one checked connection. Stop disconnects its observer,
disposes its listener-backed connection, clears the authoritative projection,
and invalidates late async results. A later start is a new client lifetime.
