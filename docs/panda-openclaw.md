# Panda / OpenClaw integration

**Branch:** `feature/panda-openclaw`. Panda is an independent AI using **OpenClaw**, not Koda's Hermes runtime. The original PandaCode desktop workspace is retained.

## Current implementation (first integration slice)

- A Rust Tauri bridge invokes OpenClaw's documented `openclaw agent --message-file - --json` interface, optionally with `--local`.
- The bridge always sets `OPENCLAW_PROFILE=panda`, separating Panda's OpenClaw configuration and state from default OpenClaw instances.
- A stable per-project session key lets the OpenClaw Gateway reuse Panda's session.
- The **Settings → Providers → Panda** workbench selects a real project directory, optional model override, session and launch mode.
- The agent is OpenClaw itself. No new LLM agent loop, homegrown MCP implementation, or fabricated skill system is introduced.
- The user's real Windows filesystem is used when OpenClaw tools execute on the Windows host, subject to that account's Windows permissions and configured OpenClaw policy.

## Setup on the user's Windows system

1. Install OpenClaw using its official supported Windows instructions, verify `openclaw --version`.
2. Configure an isolated profile for Panda using `openclaw --profile panda onboard` (confirm the options for the installed CLI).
3. Set up provider credentials/BYOK, any local model endpoint, MCP integrations, skills, and agent identity **inside Panda's profile** using OpenClaw's supported commands. Never share Hermes/Koda state or keys.
4. For a Gateway-backed session, run the Gateway under the same `panda` profile and use its normal authentication/onboarding.
5. Open PandaCode → **Settings → Providers → Panda**. Use **Check OpenClaw**, select a real project folder, and test a read-only task before permitting significant system changes.

### Environment and Windows caveat

The bridge assumes `openclaw` is a directly spawnable executable or `PANDA_OPENCLAW_EXECUTABLE` points to one. Windows npm's `.cmd` shim may not be directly spawnable via Rust's `tokio::process::Command`; support for that launcher format or resolving the bundled Node entrypoint still needs to be implemented and tested. **This has not yet been run on the user's Windows machine.**

### Boundary and permissions

OpenClaw does not require Docker in order to execute tools on the host. Set its host execution/sandbox policy through **OpenClaw's own documented configuration**; PandaCode must not silently disable security controls. For Windows native host execution, confirm that the Gateway/tool process is a Windows process rather than WSL. User privilege remains limited by the actual Windows account/token.

## What remains before this becomes Panda's default harness

1. Replace the one-turn CLI bridge with the official OpenClaw Gateway client/API, using authenticated operator scopes, device pairing, session event subscriptions and reconnect behavior. Reference: https://docs.openclaw.ai/gateway/clients and https://docs.openclaw.ai/gateway/embedding.
2. Register Panda/OpenClaw as a first-class selectable provider in PandaCode's composer, main task board, session storage, and provider model catalog.
3. Stream chat/tool/approval events, support cancel/steer/resume, and preserve artifacts/diffs.
4. Support supervising the Gateway and safe lifecycle recovery, plus Windows native launcher resolution (including npm `.cmd` shims).
5. Map skills/MCP settings directly to OpenClaw; don't duplicate them in PandaCode.
6. Build and test Windows x64, exercise local PowerShell/filesystem tools and model credentials, and verify project session isolation.

**Status:** code committed for review; not compiled or end-to-end validated. Do not merge into main or represent as production ready.


## October 21 integration verification notes

Panda is now registered as a normal provider and the OpenClaw command resolves through PandaCode's existing cross-platform launcher, including Windows npm shims. The CLI bridge returns completed responses, not live Gateway streaming. Its permission selection is deliberately labeled OpenClaw-managed; the app does not claim to enforce approvals it has not connected. The Windows build has not yet been verified, and no GitHub commit status checks have been reported for these changes.
