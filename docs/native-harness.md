# Native harness (development branch)

This fork is being extended from an external-CLI workspace into a native coding-agent host. Existing Claude Code, Codex, OpenCode, Kimi and other integrations remain unchanged.

## Implemented in this branch

- Rust-native OpenAI-compatible Chat Completions loop with tool-call continuation (up to 20 model iterations).
- Local tool interfaces: `read_file`, optional `write_file`, optional `exec_command`.
- Model endpoint and model ID per workbench conversation. API keys are stored using the existing OS secure-store wrapper, keyed by profile name; no secrets in localStorage or SQLite.
- MCP client discovery and tools/call for configured stdio JSON-RPC servers and JSON/event-stream Streamable HTTP endpoints (initial implementation).
- Skills selected by name from `SKILL.md` files under project or home skill directories.
- Interactive workbench under **Settings → Providers → Native harness**.
- History carried forward for subsequent workbench turns.

## Important limitations — not yet a full replacement harness

This is the initial vertical slice, not a verified production release. It has **not** yet been integrated as a selectable provider in the ordinary PandaCode composer/task board. Workbench conversations are kept in memory rather than automatically persisted as normal task sessions. Skills are loaded as instructions, not automatically installed with dependencies. This first MCP implementation does not cover OAuth or every Streamable HTTP event lifecycle. Execution uses the current OS user's permissions; selecting command execution is high trust. Cancellation, streamed inference, granular approvals, robust process trees, provider adapters beyond OpenAI-compatible chat completions, sandbox selection, MCP authentication, and end-to-end tests still require implementation.

**Do not release or treat as secure/unattended until validation and those protections are complete.**

## Try the workbench

1. Build the development branch with the prerequisites in the upstream README: `npm ci`, `npm run desktop`.
2. Open **Settings → Providers**, find **Native harness**.
3. Enter a profile name, full OpenAI-compatible `/v1/chat/completions` endpoint and model ID.
4. Enter an API key and choose **Save key** if the provider requires it. Keys go to the OS secure store.
5. Select your project folder and optionally enter comma-separated skill names.
6. Configure MCP servers as a JSON array. Example stdio:

```json
[{"name":"filesystem","command":"your-mcp-server-program","args":["--argument"]}]
```

Example Streamable HTTP (only trusted remote HTTPS or localhost HTTP endpoints):

```json
[{"name":"search","url":"http://127.0.0.1:3000/mcp"}]
```

7. Start with both permission toggles unchecked. Enable writes or command execution only for projects and prompts you trust. Choose **Run native agent**.

The `native_run_turn` IPC currently returns a completed turn rather than streaming events. The workbench is best suited to limited test workloads.

## Next engineering steps

1. Compile and fix the Rust/TypeScript implementation; add unit tests, mocked model HTTP tests, MCP stdio/HTTP protocol fixtures and Windows smoke tests.
2. Move native model profiles and MCP configurations to typed persistent storage with explicit host approval policy and keychain-backed secrets.
3. Implement a first-class `native` adapter through `providers::RunConfig` and `AgentEvent`, with stop/approval/streaming and existing task persistence.
4. Integrate native model discovery, provider selection, team delegation and task review into the regular app.
5. Implement complete MCP session lifecycle, pagination, cancellation, HTTP authorization/OAuth and robust transport edge cases.
6. Add skill routing, skill manifests and execution policy, startup configuration, tool permission gates, resource limits and detailed audit logging.
7. Run the complete PandaCode test matrix and package signed/tested Windows builds.

Implementation files:
- `src-tauri/src/native_harness.rs`
- `src/native-harness-panel.ts`
- Registrations in `src-tauri/src/lib.rs` and `src/provider-settings.ts`
