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
fn openclaw_command() -> Command {
    // Windows npm installs a .cmd shim which cannot be spawned as an executable
    // by Tokio directly. Resolve its installed Node entrypoint rather than
    // invoking cmd.exe /C with interpolated user input.
    #[cfg(windows)]
    let mut cmd = {
        let supplied = std::env::var_os("PANDA_OPENCLAW_EXECUTABLE");
        let candidates: Vec<PathBuf> = if let Some(path) = supplied {
            vec![PathBuf::from(path)]
        } else {
            let mut out = vec![];
            if let Some(appdata) = std::env::var_os("APPDATA") {
                out.push(PathBuf::from(appdata).join("npm").join("node_modules").join("openclaw").join("openclaw.mjs"));
            }
            if let Some(home) = std::env::var_os("USERPROFILE") {
                out.push(PathBuf::from(home).join(".local").join("bin").join("openclaw.mjs"));
                out.push(PathBuf::from(home).join("openclaw").join("openclaw.mjs"));
            }
            out
        };
        let file = candidates.iter().find_map(|path| {
            if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("cmd")) {
                let sibling = path.parent()?.join("node_modules").join("openclaw").join("openclaw.mjs");
                if sibling.is_file() { return Some(sibling); }
            }
            if path.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("mjs") || ext.eq_ignore_ascii_case("js")) {
                return Some(path.clone());
            }
            None
        });
        if let Some(entry) = file {
            let mut command = Command::new("node");
            command.arg(entry);
            command
        } else {
            Command::new("openclaw.exe")
        }
    };
    #[cfg(not(windows))]
    let mut cmd = Command::new(std::env::var_os("PANDA_OPENCLAW_EXECUTABLE").unwrap_or_else(|| "openclaw".into()));
    cmd.arg("--profile").arg(PROFILE);
    cmd.env("OPENCLAW_DISABLE_BONJOUR", "1");
    cmd
}

#[tauri::command]
pub async fn panda_openclaw_check() -> Result<String,String> {
    let result=tokio::time::timeout(Duration::from_secs(12), openclaw_command().arg("--version").output())
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
    let mut cmd=openclaw_command();
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
    if result.stdout.len()>MAX_JSON {return Err("OpenClaw output exceeded limit".into())}
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
    Ok(PandaReply{text,raw,profile:PROFILE})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn check_id(){assert!(validate_id("agent:panda:project-1").is_ok()); assert!(validate_id("../escape").is_err()); assert!(validate_model("openai/gpt-5").is_ok()); assert!(validate_model("../bad").is_err());}
}
