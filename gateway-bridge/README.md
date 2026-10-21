# Panda Gateway bridge (experimental)

This is the **official SDK based** streaming path for Panda. It is built on OpenClaw's existing agent harness, not a replacement inference/tool engine.

## Install

From the PandaCode checkout:

```powershell
cd gateway-bridge
npm install
cd ..
```

Requires Node >=22.19.0, a running OpenClaw Gateway for the **panda** profile, and authentication configured according to OpenClaw's documented Gateway setup. Set `PANDA_GATEWAY_TOKEN` in the native application's environment only for an authorized, local Gateway. Never embed this credential in PandaCode source or browser localStorage.

For a development desktop build, set `PANDA_USE_GATEWAY=1` to select the Gateway streaming path; unset it to use the older completed-turn OpenClaw CLI fallback. The bridge defaults to `ws://127.0.0.1:18789`, so configure Panda's Gateway to use that port, or extend the host configuration before switching to a different port.

The SDK dependency versions are exact-pinned in `gateway-bridge/package.json`. **The workspace has no installed bridge dependencies and no generated lockfile committed yet.** Do not use `npm ci` in that folder until a lockfile is produced and reviewed. The regular PandaCode root npm install is unaffected.

## State of implementation

- Rust spawns a Node-based SDK bridge for Panda turns when `PANDA_USE_GATEWAY=1`.
- The official `GatewayClient` connects and waits for `hello-ok`.
- The bridge calls `chat.send` with an idempotency key, translates chat text to JSONL updates, and attempts `chat.abort` on cancellation.
- Session key is scoped to `agent:panda:<PandaCode task id>`; provider sessions are preserved separately from Koda/Hermes.
- Structured tool events are detected, but a complete PandaCode tool-row mapper and approval UI have **not** yet been implemented.
- Device pairing and persistent operator credentials must be integrated through official host callbacks before release.
- Reconnect reconciliation, snapshot replacement, user-facing Gateway URL configuration, Windows packaging, lockfile, and end-to-end tests are still needed.

This is committed **development code**, not a claimed successfully connected Gateway. Keep PR #2 in draft until live Windows proof is collected.

Primary protocol references: https://docs.openclaw.ai/gateway/clients and https://docs.openclaw.ai/gateway/protocol .
