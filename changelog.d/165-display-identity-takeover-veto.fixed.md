A monitor with a different hardware UUID can no longer take over a known
display identity through a weaker correlation tier. When both sides carry
non-empty strong key sets with nothing in common, adapter, exact geometry
and scale, and weak matching are all vetoed, so a reused GPUI adapter
`id:` or a same-geometry swap allocates a new display instead of merging
both UUIDs into one identity. Retained evidence now unions only
non-conflicting strong keys and replaces adapter and weak keys with the
latest observation, so stale attachment descriptions stop widening later
weaker-tier matches. Contract 009 records the veto.
