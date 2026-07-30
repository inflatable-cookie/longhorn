# longhorn-command

Pure command declaration, context, argument, registry, availability, fresh
admission, injected execution-port, physical keyboard, immutable effective
keymap, conflict, discovery, and search primitives.

The crate owns structural contracts only. Consumers retain command meaning,
runtime context facts, availability rules, authorization, execution routes,
platform-reserved policy, and product receipts. It has no config, Tauri,
bridge, settings, renderer, browser, or product dependency.

V1 keyboard input is one press-only physical chord. Versioned presets combine
with sparse disable, replace, and add directives. Resolution uses one validated
hot-context path, most-specific context wins, and equal-rank different
invocations remain explicit unconsumed conflicts.
