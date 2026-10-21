//! Panda's OpenClaw bridge. OpenClaw owns model inference, tools, skills, MCP and memory.
//! This module only launches its documented CLI in Panda's isolated OpenClaw profile.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::{io::AsyncWriteExt, process::Command};

const PROFILE: &str = "panda";
const MAX_PROMPT: usize = 1024 * 1024;
const MAX_JSON: usize = 8 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PandaRequest {
    pub project_dir: String,
    pub prompt: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub session_key: Option<String>,
    #[serde(default)]
    pub direct: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PandaReply {
    pub text: String,
    pub raw: Value,
    pub profile: &'static str,
}
fn validate_id(value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 160 || !value.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b,b'_' | b'-' | b'.' | b':' )) {
        return Err("Invalid Panda session or model identifier".into())
    }
    Ok(())
}
fn validate_model(value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 200 || !value.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':' | b'/' )) {
        return Err("Invalid model identifier".into());
    }
    Ok(())
}
fn root(path: &str) -> Result<PathBuf, String> {
    let dir = std::fs::canonicalize(path).map_err(|_| "Project directory was not found")?;
    if !dir.is_dir() {return Err("Project path is not a folder".into())}
    Ok(dir)
}
pub(crate) fn available() -> bool {
    resolve_executable().is_some()
}
fn resolve_executable() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PANDA_OPENCLAW_EXECUTABLE") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }
    crate::providers::tool_path("openclaw")
}
fn openclaw_command() -> Result<Command, String> {
    let path = resolve_executable().ok_or("OpenClaw executable not found. Install OpenClaw or configure PANDA_OPENCLAW_EXECUTABLE")?;
    let mut cmd = crate::providers::command(&path)?;
    cmd.arg("--profile").arg(PROFILE);
    cmd.env("OPENCLAW_DISABLE_BONJOUR", "1");
    Ok(cmd)
}

#[tauri::command]
pub async fn panda_openclaw_check() -> Result<String,String> {
    let mut cmd = openclaw_command()?;
    let result=tokio::time::timeout(Duration::from_secs(12), cmd.arg("--version").output())
        .await.map_err(|_|"OpenClaw version check timed out")?.map_err(|e|format!("OpenClaw not available: {e}"))?;
    if !result.status.success(){return Err("OpenClaw CLI version check failed".into())}
    Ok(String::from_utf8_lossy(&result.stdout).trim().to_owned())
}
#[tauri::command]
pub async fn panda_openclaw_turn(req:PandaRequest) -> Result<PandaReply,String> {
    let project=root(&req.project_dir)?;
    if req.prompt.trim().is_empty() || req.prompt.len()>MAX_PROMPT {return Err("Message is empty or too large".into())}
    if let Some(model)=req.model.as_deref(){validate_model(model)?;}
    if let Some(key)=req.session_key.as_deref(){validate_id(key)?;}
    let mut cmd=openclaw_command()?;
    cmd.current_dir(project)
       .arg("agent").arg("--agent").arg("main")
       .arg("--message-file").arg("-")
       .arg("--json")
       .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
       .kill_on_drop(true);
    if req.direct {cmd.arg("--local");}
    if let Some(model)=req.model {cmd.arg("--model").arg(model);}
    if let Some(key)=req.session_key {cmd.arg("--session-key").arg(key);}
    let mut child=cmd.spawn().map_err(|e|format!("Cannot launch OpenClaw: {e}"))?;
    let mut input=child.stdin.take().ok_or("OpenClaw stdin unavailable")?;
    input.write_all(req.prompt.as_bytes()).await.map_err(|_|"Unable to send Panda prompt")?;
    drop(input);
    let result=tokio::time::timeout(Duration::from_secs(900), child.wait_with_output())
        .await.map_err(|_|"OpenClaw turn exceeded 15 minutes")?
        .map_err(|e|format!("OpenClaw process failed: {e}"))?;
    if result.stdout.len()>MAX_JSON || result.stderr.len()>MAX_JSON {return Err("OpenClaw output exceeded limit".into())}
    let output:String=String::from_utf8(result.stdout).map_err(|_|"OpenClaw output was not UTF-8")?;
    if !result.status.success(){
        let error=String::from_utf8_lossy(&result.stderr);
        let error:String=error.chars().take(1200).collect();
        return Err(format!("OpenClaw returned {}: {}",result.status,error));
    }
    let raw:Value=serde_json::from_str(&output).map_err(|_|"OpenClaw did not return its expected JSON result")?;
    let text=raw["result"]["payloads"].as_array()
      .or_else(||raw["payloads"].as_array())
      .map(|items| items.iter().filter_map(|x|x["text"].as_str()).collect::<Vec<_>>().join("\n"))
      .filter(|s|!s.is_empty())
      .or_else(||raw["final"].as_str().map(str::to_owned))
      .or_else(||raw["result"]["final"].as_str().map(str::to_owned))
      .unwrap_or_default();
    if text.trim().is_empty() && !raw["ok"].as_bool().unwrap_or(true) {
        return Err("OpenClaw reported an unsuccessful turn without a final reply".into());
    }
    Ok(PandaReply{text,raw,profile:PROFILE})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn check_id(){assert!(validate_id("agent:panda:project-1").is_ok()); assert!(validate_id("../escape").is_err()); assert!(validate_model("openai/gpt-5").is_ok()); assert!(validate_model("../bad").is_err());}
    #[test]
    fn reject_invalid_model_and_session_ids() {
        for bad in ["", "../oops", "x y", "slash/name", "a".repeat(161).as_str()] {
            assert!(validate_id(bad).is_err(), "unexpected session identifier {bad}");
        }
        for valid in ["ollama/qwen3.5:9b", "anthropic/claude-sonnet-4-6", "openai/gpt-6"] {
            assert!(validate_model(valid).is_ok(), "valid provider model {valid}");
        }
    }
    #[test]
    fn reject_invalid_project_paths() {
        assert!(root("/this/pandacode/folder/does/not/exist").is_err());
    }

}

pub(crate) async fn gateway_turn(
    session_id: String,
    project_dir: PathBuf,
    prompt: String,
    sink: crate::providers::Sink,
    mut input: tokio::sync::mpsc::UnboundedReceiver<crate::providers::RunInput>,
) -> Result<(), String> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use crate::providers::{AgentEvent, RunInput};
    let bridge = std::env::var_os("PANDA_GATEWAY_BRIDGE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("gateway-bridge").join("panda-gateway.mjs"));
    if !bridge.is_file() { return Err(format!("Gateway bridge not found: {}", bridge.display())); }
    let mut command = Command::new("node");
    command.arg(&bridge).current_dir(&project_dir)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|e| format!("Cannot start Gateway bridge: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("Gateway control input missing")?;
    let stdout = child.stdout.take().ok_or("Gateway event output missing")?;
    let key = format!("agent:panda:{}", session_id);
    let request = serde_json::json!({
        "sessionKey":key,
        "prompt": prompt,
        "url":"ws://127.0.0.1:18789"
    });
    stdin.write_all(format!("{request}\n").as_bytes()).await.map_err(|e|e.to_string())?;
    let mut reader = BufReader::new(stdout).lines();
    let mut complete = false;
    let mut accepted = false;
    let mut message_started = false;
    let mut emitted_text = String::new();
    let mut error = None;
    loop {
        tokio::select! {
            next = reader.next_line() => {
                let Some(line) = next.map_err(|e|e.to_string())? else { break };
                if line.len()>256*1024 { return Err("Gateway event too large".into()); }
                let Ok(event) = serde_json::from_str::<Value>(&line) else { continue };
                let kind=event["kind"].as_str().unwrap_or_default();
                match kind {
                    "text" | "snapshot" => {
                        let text=event["text"].as_str().unwrap_or("");
                        if !message_started {
                            sink(AgentEvent{session_id:session_id.clone(),kind:"message_start".into(),text:String::new(),request_id:None,data:Some(serde_json::json!({"messageId":session_id})),tool:None});
                            message_started=true;
                        }
                        let delta = if kind == "snapshot" {
                            if !text.starts_with(&emitted_text) {
                                error = Some("Gateway text baseline changed; reconnect recovery is required".into());
                                break;
                            }
                            let suffix = &text[emitted_text.len()..];
                            emitted_text = text.to_owned();
                            suffix.to_owned()
                        } else {
                            emitted_text.push_str(text);
                            text.to_owned()
                        };
                        if !delta.is_empty() { sink(AgentEvent{session_id:session_id.clone(),kind:"text".into(),text:delta,request_id:None,data:None,tool:None}); }
                    }
                    "status" => sink(AgentEvent{session_id:session_id.clone(),kind:"status".into(),text:event["text"].as_str().unwrap_or("").into(),request_id:None,data:None,tool:None}),
                    "tool" => {
                        let name=event["data"]["tool"].as_str().unwrap_or("OpenClaw tool");
                        let phase=event["data"]["phase"].as_str().unwrap_or("working");
                        sink(AgentEvent { session_id:session_id.clone(), kind:"status".into(),
                            text:format!("{name}: {phase}"), request_id:None,
                            data:Some(event["data"].clone()), tool:None });
                    },
                    "accepted" => {
                        accepted=true;
                        if !message_started {
                            sink(AgentEvent{session_id:session_id.clone(),kind:"message_start".into(),text:String::new(),request_id:None,data:Some(serde_json::json!({"messageId":event["runId"]})),tool:None});
                            message_started=true;
                        }
                    },
                    "complete" => {complete=true;break;},
                    "aborted" => return Err("Panda Gateway turn cancelled".into()),
                    "error" => {error=Some(event["text"].as_str().unwrap_or("Gateway failed").to_owned());break;},
                    _ => {}
                }
            },
            incoming = input.recv() => match incoming {
                Some(RunInput::Stop) | None => {
                    let _=stdin.write_all(b"{\"kind\":\"abort\"}\n").await;
                    let _=tokio::time::timeout(Duration::from_secs(5),child.wait()).await;
                    return Err("Panda Gateway turn cancelled".into());
                },
                Some(RunInput::Steer { reply, .. }) => {let _=reply.send(Err("Gateway steering is not wired yet".into()));},
                Some(RunInput::Goal {reply,..}) => {let _=reply.send(Err("Panda goal tools need Gateway mapping".into()));},
                Some(RunInput::CodexRequest {reply,..}) => {let _=reply.send(Err("Not a Codex provider".into()));},
                Some(RunInput::UserInput {reply,..}) => {if let Some(reply)=reply {let _=reply.send(Err("Unsupported user input".into()));}},
                Some(RunInput::Approval {..}) => {}
            }
        }
    }
    let status = child.wait().await.map_err(|e|e.to_string())?;
    if let Some(error)=error {return Err(error);}
    if !complete || !accepted || !status.success() {return Err("Gateway turn did not complete successfully".into());}
    Ok(())
}
