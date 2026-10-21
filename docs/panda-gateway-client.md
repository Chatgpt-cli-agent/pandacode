# Official OpenClaw Gateway client integration (Panda)

Panda is a standalone AI identity with its own OpenClaw **panda** profile, independent of Koda/Hermes. OpenClaw owns model inference, skills, MCP tools, memory and execution; PandaCode owns the desktop experience.

## Decision: official packages, not another protocol implementation

Pin the Gateway packages at the documented verified stable release:

```bash
npm install --save-exact @openclaw/gateway-client@2026.8.1 @openclaw/gateway-protocol@2026.8.1
```

Use the official `@openclaw/gateway-client` Node host implementation or browser-safe client with a secure Tauri host adapter. Do not implement WebSocket signatures, reconnect, identity, token rotation, and approval handling independently. The protocol is v4 in this release; package version is NOT wire version.

Primary references:
- https://docs.openclaw.ai/gateway/clients
- https://docs.openclaw.ai/gateway/embedding
- https://docs.openclaw.ai/gateway/protocol/handshake
- https://docs.openclaw.ai/gateway/protocol/rpc-bootstrap-and-events
- https://docs.openclaw.ai/gateway/operator-scopes

## Connection design

1. Discover or supervise a Gateway started explicitly under the `panda` OpenClaw profile. Verify it really runs on the Windows host; do not silently route tool execution to WSL or a VM.
2. Initialize an official Gateway client with a persistent **Panda-specific** device keypair and token storage kept via the existing Rust OS credential store. Never return bootstrap secrets or device private keys to the renderer.
3. Complete `connect.challenge` and `hello-ok` with the official client's device-signing, pairing and reconnect logic. Treat `hello-ok` as readiness, not an open TCP port.
4. Request the minimum required operator scopes: `operator.read`, `operator.write`, and (for interactive approvals) `operator.approvals`. Enable additional scopes only when a concrete UI feature requires them.
5. Use `sessions.subscribe` with a populated `sessions.list` request, `chat.history` for recovery, and `chat.send` or `agent` for turns. Use idempotency keys on mutating RPCs.
6. Render `chat` delta events carefully: the first `message` snapshot already contains `deltaText`. Never append that delta twice. Distinguish `replace=true` from append.
7. Associate incoming `agent`/`session.tool` events with their PandaCode session and run IDs. Normalize to existing `AgentEvent` without losing approval request IDs.
8. Handle stop/interruption through supported Gateway RPC, not only by killing the client connection. Reconcile pending runs and message history after reconnect.
9. Apply OpenClaw's actual sandbox and tool policy (configured by user); do not imply PandaCode's permission dropdown overrides that policy.
10. Run compatibility checks against the exact Gateway release used for tests. Upgrade official packages together with tested OpenClaw CLI/Gateway.

## Transition

The existing `panda_openclaw.rs` CLI adapter is a usable **development fallback** for completed turns; it is not live streaming. Keep it separate until the official SDK connection passes end-to-end tests. No homegrown inference, memory, tool execution, or MCP implementation is needed.

## Acceptance criteria

- Windows x64 native build passes, and `openclaw --profile panda --version` is spawnable through the existing launcher.
- An authenticated client connects to an OpenClaw Gateway using device pairing and scoped token storage.
- A task sends a prompt and displays streamed text and tool activity with no duplicated deltas.
- Restart/reconnect recovers Panda's sessions and incomplete turns without duplicate execution.
- Stop, approvals, and errors follow their documented protocol behavior.
- BYOK credentials, skills, MCP and Windows host tools work inside the `panda` profile.
- Koda's Hermes models, skills, memory and credentials are never accessed or reused.
- CI passes TypeScript tests and Rust checks; Windows smoke tests are recorded.
