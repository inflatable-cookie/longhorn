# Bovine request: production MCP infrastructure

Status: operator-authorised consumer request; Longhorn planning settlement needed
Requester: Acowtancy Desktop Chatterbox, Tom direction 2026-09-06

Tom explicitly requests a production version of the MCP server infrastructure
for Bovine's contextual agent. No Longhorn runtime implementation, dependency
change, tag, release or publication is authorised by this intake alone.

## Need

Bovine agents need structured app context and tools in client release builds.
Contract 022's dev-only server is valuable test infrastructure, but release
builds exclude it and evaluate/command expose code execution. Do not simply
enable the development feature in production.

Consumer specification: acowtancy/bovine-accelerator-desktop,
docs/specs/010-contextual-chat-and-task-queue.md. It defines per-thread viewing
context, stable per-task editing targets, durable queued requests and explicit
steering. Content skills remain in the Bovine content repository.

## Requested shared boundary

- Separately opted-in production server and typed host tool/resource registry.
- Authenticated local transport, origin validation when HTTP, per-instance
  credential bootstrap, scoped admission/revocation and stale-instance cleanup.
- Supported harness connection without global CLI configuration changes or
  exposing credentials in transcripts, content repositories or logs.
- Host-supplied workspace/thread/task/attempt admission context; reject stale
  or foreign calls before execution. Longhorn does not interpret Bovine IDs.
- Bounded request/response sizes, concurrency, cancellation, clear typed errors,
  redacted diagnostics and deterministic lifecycle cleanup.
- No inherited evaluate, synthetic input, shell or arbitrary app-command bridge.

Desktop owns domain schemas and tool implementations, content permissions,
proposal/Save transactions, Git, task scheduling and skills. Proposed first
tools are get_context, search_content, read_content and navigate/show_content.
Later writes use existing consumer transactions; transport does not grant them.
Swallowtail owns registering the connection through each supported adapter.

## Return capsule

Please return the proposed public registration/transport API, capability and
credential lifecycle, exact crate/feature ownership, supported host/harness
integration, production consumer example, acceptance checks and release
dependency. Identify any product decision requiring Tom rather than guessing.

Acceptance must include a release-build consumer invoking one registered tool,
wrong-instance/expired/revoked/foreign-context refusals, bounded cancellation
and shutdown, absence of dev code-execution tools, and no credential leakage.
Use a generic consumer fixture; do not move Bovine product policy into Longhorn.

## Placement and disposition

At request capture, Paseo returned no live agents rooted in the Longhorn checkout
and no active Longhorn workspace. No identity was revived or replacement created.
Route this intake to Longhorn's next live planning owner. Promote into its
contract/roadmap after settlement and remove this note on full promotion.
