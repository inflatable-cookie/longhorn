Native-content detach targets the observed generation and can settle a failed predecessor after desired state advances. A detached generation cannot attach again. Decoded plans validate their step count, order, and dependencies. All three mechanism adapters share apply and attach sequencing. Backing surfaces release handles returned after host destruction or retain them for teardown retry. Tauri closes a new child when its initial hide fails.

### Breaking

- **Native-content renderer protocol v2.** Upgrade the Rust host and generated TypeScript client together. Custom snapshot, desired-update, and content-size decision requests must carry the `authority_epoch` returned by connect. Give each replacement host a fresh, increasing epoch. Handle the new `AuthorityEpochMismatch` rejection by reconnecting.
- **Child-view focus is explicit.** Plan focus intent applies to mechanisms that declare observable focus. For a Tauri child view, call `ChildViewAdapter::request_focus(generation)` after attach when focus is needed, and submit `FocusIntent::Unchanged` in the plan.
- **Confirmed detach retires the generation.** Advance to the next generation on re-present. Same-generation present updates return `TerminalGeneration`.
