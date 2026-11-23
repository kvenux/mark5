use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::mcp_server::{resolve_agy_bin, ActiveSessionContext, DocMcpServer};
use crate::types::{AgentStreamEvent, SendAgentTurnPayload};
use crate::workspace::WorkspaceManager;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub type EventCallback = Arc<dyn Fn(AgentStreamEvent) + Send + Sync>;

pub struct AgyAdapter {
    mcp: Arc<DocMcpServer>,
    workspace: Arc<WorkspaceManager>,
    active_procs: Mutex<HashMap<String, u32>>,
}

impl AgyAdapter {
    pub fn new(mcp: Arc<DocMcpServer>, workspace: Arc<WorkspaceManager>) -> Arc<Self> {
        Arc::new(Self {
            mcp,
            workspace,
            active_procs: Mutex::new(HashMap::new()),
        })
    }

    pub async fn abort(&self, session_id: &str) {
        let pid_opt = {
            let mut map = self.active_procs.lock().await;
            map.remove(session_id)
        };
        if let Some(pid) = pid_opt {
            kill_pid(pid).await;
        }
    }

    pub async fn abort_all(&self) {
        let pids: Vec<u32> = {
            let mut map = self.active_procs.lock().await;
            map.drain().map(|(_, pid)| pid).collect()
        };
        for pid in pids {
            kill_pid(pid).await;
        }
    }

    pub async fn run_turn(
        &self,
        req: SendAgentTurnPayload,
        engine: String,
        conversation_id: Option<String>,
        on_event: EventCallback,
    ) -> Result<(), String> {
        self.abort(&req.session_id).await;

        self.mcp
            .set_active_context(ActiveSessionContext {
                session_id: req.session_id.clone(),
                root_path: req.root_path.clone(),
                doc_rel_path: req.doc_rel_path.clone(),
            })
            .await;

        let engine_norm = engine.to_lowercase();
        let engine_display = match engine_norm.as_str() {
            "claude" => "Claude Code",
            "codex" => "Codex",
            _ => "AGY",
        };

        let full_prompt = build_prompt(&req, &self.mcp.server_name);
        let mcp_url = self.mcp.get_url().await;

        on_event(make_status_event(
            &req.session_id,
            &format!("正在连接 {engine_display}..."),
        ));

        let (mut child, bin_label) = match engine_norm.as_str() {
            "claude" => spawn_claude(&req, &full_prompt, &self.mcp.server_name, &mcp_url, conversation_id.as_deref())?,
            "codex" => spawn_codex(&req, &full_prompt, &self.mcp.server_name, &mcp_url, conversation_id.as_deref()).await?,
            _ => spawn_agy(&req, &full_prompt, conversation_id.as_deref())?,
        };

        let child_pid = child.id().unwrap_or(0);
        if child_pid > 0 {
            self.active_procs
                .lock()
                .await
                .insert(req.session_id.clone(), child_pid);
        }

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let stderr_handle = tokio::spawn(async move {
            let mut buf = String::new();
            if let Some(se) = stderr {
                let mut reader = BufReader::new(se).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    buf.push_str(&line);
                    buf.push('\n');
                }
            }
            buf
        });

        let mut accumulated = String::new();
        let mut saw_result = false;
        let mut step_counter: i64 = 0;
        let mut claude_tool_steps: HashMap<String, (i64, String)> = HashMap::new();
        let mut codex_item_steps: HashMap<String, (i64, String)> = HashMap::new();
        let mut saw_claude_delta = false;

        if let Some(so) = stdout {
            let mut reader = BufReader::new(so).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let trimmed = line.trim();
                if !trimmed.starts_with('{') {
                    continue;
                }
                let parsed: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                match engine_norm.as_str() {
                    "claude" => {
                        self.handle_claude_line(
                            &parsed,
                            &req,
                            &on_event,
                            &mut accumulated,
                            &mut saw_result,
                            &mut step_counter,
                            &mut claude_tool_steps,
                            &mut saw_claude_delta,
                        )
                        .await;
                    }
                    "codex" => {
                        self.handle_codex_line(
                            &parsed,
                            &req,
                            &on_event,
                            &mut accumulated,
                            &mut saw_result,
                            &mut step_counter,
                            &mut codex_item_steps,
                        )
                        .await;
                    }
                    _ => {
                        self.handle_agy_line(
                            &parsed,
                            &req,
                            &on_event,
                            &mut accumulated,
                            &mut saw_result,
                        )
                        .await;
                    }
                }
            }
        }

        let status = child.wait().await;
        let stderr_buf = stderr_handle.await.unwrap_or_default();

        let still_current = {
            let mut map = self.active_procs.lock().await;
            if map.get(&req.session_id).copied() == Some(child_pid) {
                map.remove(&req.session_id);
                true
            } else {
                false
            }
        };

        if still_current && !saw_result {
            let code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
            if code == 0 {
                on_event(make_done_event(&req.session_id, accumulated));
            } else {
                let err = if stderr_buf.trim().is_empty() {
                    format!("{engine_display} ({bin_label}) 进程异常退出 (code={code})")
                } else {
                    stderr_buf.trim().to_string()
                };
                on_event(make_error_event(&req.session_id, err));
            }
        }

        Ok(())
    }

    async fn handle_agy_line(
        &self,
        parsed: &Value,
        req: &SendAgentTurnPayload,
        on_event: &EventCallback,
        accumulated: &mut String,
        saw_result: &mut bool,
    ) {
        let ev_name = parsed.get("event").and_then(|v| v.as_str()).unwrap_or("");
        match ev_name {
            "init" => {
                let cid = parsed
                    .get("conversation_id")
                    .or_else(|| parsed.pointer("/init/conversation_id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                if let Some(ref c) = cid {
                    let _ = self
                        .workspace
                        .update_session_conversation_id(&req.root_path, &req.session_id, c)
                        .await;
                }
                on_event(make_init_event(&req.session_id, cid));
            }
            "step_update" => {
                let Some(su) = parsed.get("step_update") else {
                    return;
                };
                let step_type = su.get("step_type").and_then(|v| v.as_str()).unwrap_or("");
                let su_state = su.get("state").and_then(|v| v.as_str()).unwrap_or("");
                let su_cid = su
                    .get("conversation_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                if step_type == "tool" {
                    let raw_name = su
                        .get("tool_name")
                        .or_else(|| su.pointer("/tool_info/name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("tool");
                    let params = su.pointer("/tool_info/parameters");
                    let mcp_tool = if raw_name == "call_mcp_tool" {
                        params
                            .and_then(|p| p.get("ToolName"))
                            .and_then(|v| v.as_str())
                            .unwrap_or(raw_name)
                    } else {
                        raw_name
                    };
                    let mcp_args = if raw_name == "call_mcp_tool" {
                        params.and_then(|p| p.get("Arguments")).or(params)
                    } else {
                        params
                    };
                    let path_arg = mcp_args
                        .and_then(|a| {
                            a.get("Path")
                                .or_else(|| a.get("AbsolutePath"))
                                .or_else(|| a.get("file_path"))
                                .or_else(|| a.get("CommandLine"))
                        })
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if is_internal_mcp_schema_read(mcp_tool, path_arg) {
                        return;
                    }

                    let state = match su_state {
                        "DONE" | "COMPLETED" | "done" => "done",
                        "ERROR" | "FAILED" | "error" => "error",
                        _ => "running",
                    };
                    let readable = format_readable_tool(mcp_tool, path_arg);
                    on_event(AgentStreamEvent {
                        session_id: req.session_id.clone(),
                        kind: "tool".to_string(),
                        conversation_id: su_cid,
                        step_index: su.get("step_index").and_then(|v| v.as_i64()),
                        tool_name: Some(mcp_tool.to_string()),
                        tool_state: Some(state.to_string()),
                        tool_detail: Some(readable),
                        duration_sec: su.get("duration_seconds").and_then(|v| v.as_f64()),
                        text: None,
                        error: None,
                    });
                } else if let Some(delta) = su.get("text_delta").and_then(|v| v.as_str()) {
                    if !delta.is_empty() {
                        accumulated.push_str(delta);
                        on_event(make_delta_event(&req.session_id, delta));
                    }
                } else if step_type == "agent_response" && su_state == "ACTIVE" {
                    on_event(make_status_event(&req.session_id, "正在思考与组织内容..."));
                }
            }
            "tool" => {
                let raw_name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("tool");
                let detail_raw = parsed.get("detail").and_then(|v| v.as_str()).unwrap_or("");
                if is_internal_mcp_schema_read(raw_name, detail_raw) {
                    return;
                }
                let state = match parsed.get("state").and_then(|v| v.as_str()) {
                    Some("done") | Some("completed") => "done",
                    Some("error") | Some("failed") => "error",
                    _ => "running",
                };
                let readable = format_readable_tool(raw_name, detail_raw);
                on_event(AgentStreamEvent {
                    session_id: req.session_id.clone(),
                    kind: "tool".to_string(),
                    conversation_id: None,
                    step_index: parsed.get("step").and_then(|v| v.as_i64()),
                    tool_name: Some(raw_name.to_string()),
                    tool_state: Some(state.to_string()),
                    tool_detail: Some(readable),
                    duration_sec: parsed.get("duration").and_then(|v| v.as_f64()),
                    text: None,
                    error: None,
                });
            }
            "delta" | "message" | "text" => {
                if let Some(chunk) = parsed
                    .get("text")
                    .or_else(|| parsed.get("content"))
                    .and_then(|v| v.as_str())
                {
                    accumulated.push_str(chunk);
                    on_event(make_delta_event(&req.session_id, chunk));
                }
            }
            "result" => {
                *saw_result = true;
                if let Some(res_obj) = parsed.get("result").and_then(|v| v.as_object()) {
                    let cid = res_obj
                        .get("conversation_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    if let Some(ref c) = cid {
                        let _ = self
                            .workspace
                            .update_session_conversation_id(&req.root_path, &req.session_id, c)
                            .await;
                    }
                    let status = res_obj.get("status").and_then(|v| v.as_str()).unwrap_or("SUCCESS");
                    if status != "SUCCESS" {
                        let err_msg = res_obj
                            .get("error")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("AGY 返回状态：{status}"));
                        on_event(make_error_event(&req.session_id, err_msg));
                        return;
                    }
                    let resp = res_obj
                        .get("response")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let final_text = if !accumulated.trim().is_empty() {
                        accumulated.trim().to_string()
                    } else {
                        resp.trim().to_string()
                    };
                    on_event(AgentStreamEvent {
                        session_id: req.session_id.clone(),
                        kind: "done".to_string(),
                        conversation_id: cid,
                        step_index: None,
                        tool_name: None,
                        tool_state: None,
                        tool_detail: None,
                        duration_sec: res_obj.get("duration_seconds").and_then(|v| v.as_f64()),
                        text: Some(final_text),
                        error: None,
                    });
                } else {
                    let final_text = parsed
                        .get("result")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| accumulated.trim().to_string());
                    on_event(make_done_event(&req.session_id, final_text));
                }
            }
            "error" => {
                *saw_result = true;
                let err_msg = parsed
                    .get("error")
                    .or_else(|| parsed.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("AGY 执行失败")
                    .to_string();
                on_event(make_error_event(&req.session_id, err_msg));
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_claude_line(
        &self,
        parsed: &Value,
        req: &SendAgentTurnPayload,
        on_event: &EventCallback,
        accumulated: &mut String,
        saw_result: &mut bool,
        step_counter: &mut i64,
        tool_steps: &mut HashMap<String, (i64, String)>,
        saw_delta: &mut bool,
    ) {
        let ev_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match ev_type {
            "system" => {
                if parsed.get("subtype").and_then(|v| v.as_str()) == Some("init") {
                    let sid = parsed
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    if let Some(ref c) = sid {
                        let _ = self
                            .workspace
                            .update_session_conversation_id(&req.root_path, &req.session_id, c)
                            .await;
                    }
                    on_event(make_init_event(&req.session_id, sid));
                }
            }
            "stream_event" => {
                if let Some(ev) = parsed.get("event") {
                    let inner = ev.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if inner == "content_block_delta" {
                        if let Some(txt) = ev.pointer("/delta/text").and_then(|v| v.as_str()) {
                            *saw_delta = true;
                            accumulated.push_str(txt);
                            on_event(make_delta_event(&req.session_id, txt));
                        }
                    } else if inner == "content_block_start" {
                        if ev.pointer("/content_block/type").and_then(|v| v.as_str()) == Some("tool_use") {
                            let id = ev
                                .pointer("/content_block/id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let name = ev
                                .pointer("/content_block/name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("tool")
                                .to_string();
                            if !id.is_empty() && !tool_steps.contains_key(&id) {
                                *step_counter += 1;
                                let step = *step_counter;
                                tool_steps.insert(id, (step, name.clone()));
                                on_event(make_tool_event(
                                    &req.session_id,
                                    step,
                                    &name,
                                    "running",
                                    &format_readable_tool(&name, ""),
                                ));
                            }
                        }
                    }
                }
            }
            "assistant" => {
                if let Some(blocks) = parsed.pointer("/message/content").and_then(|v| v.as_array()) {
                    for b in blocks {
                        let bt = b.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        if bt == "tool_use" {
                            let id = b.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let name = b.get("name").and_then(|v| v.as_str()).unwrap_or("tool").to_string();
                            if !id.is_empty() && !tool_steps.contains_key(&id) {
                                *step_counter += 1;
                                let step = *step_counter;
                                tool_steps.insert(id, (step, name.clone()));
                                on_event(make_tool_event(
                                    &req.session_id,
                                    step,
                                    &name,
                                    "running",
                                    &format_readable_tool(&name, ""),
                                ));
                            }
                        } else if bt == "text" && !*saw_delta {
                            if let Some(txt) = b.get("text").and_then(|v| v.as_str()) {
                                accumulated.push_str(txt);
                                on_event(make_delta_event(&req.session_id, txt));
                            }
                        }
                    }
                }
            }
            "user" => {
                if let Some(blocks) = parsed.pointer("/message/content").and_then(|v| v.as_array()) {
                    for b in blocks {
                        if b.get("type").and_then(|v| v.as_str()) == Some("tool_result") {
                            let tid = b.get("tool_use_id").and_then(|v| v.as_str()).unwrap_or("");
                            let is_err = b.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
                            if let Some((step, name)) = tool_steps.get(tid) {
                                on_event(make_tool_event(
                                    &req.session_id,
                                    *step,
                                    name,
                                    if is_err { "error" } else { "done" },
                                    &format_readable_tool(name, ""),
                                ));
                            }
                        }
                    }
                }
            }
            "result" => {
                *saw_result = true;
                if let Some(sid) = parsed.get("session_id").and_then(|v| v.as_str()) {
                    let _ = self
                        .workspace
                        .update_session_conversation_id(&req.root_path, &req.session_id, sid)
                        .await;
                }
                let is_err = parsed.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
                let result_txt = parsed
                    .get("result")
                    .and_then(|v| v.as_str())
                    .unwrap_or(accumulated.as_str())
                    .to_string();
                if is_err {
                    on_event(make_error_event(&req.session_id, result_txt));
                } else {
                    on_event(make_done_event(&req.session_id, result_txt));
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_codex_line(
        &self,
        parsed: &Value,
        req: &SendAgentTurnPayload,
        on_event: &EventCallback,
        accumulated: &mut String,
        saw_result: &mut bool,
        step_counter: &mut i64,
        item_steps: &mut HashMap<String, (i64, String)>,
    ) {
        let ev_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match ev_type {
            "thread.started" => {
                let tid = parsed
                    .get("thread_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                if let Some(ref c) = tid {
                    let _ = self
                        .workspace
                        .update_session_conversation_id(&req.root_path, &req.session_id, c)
                        .await;
                }
                on_event(make_init_event(&req.session_id, tid));
            }
            "item.started" => {
                if let Some(item) = parsed.get("item") {
                    let itype = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if itype == "mcp_tool_call" || itype == "command_execution" {
                        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let name = item
                            .get("tool")
                            .or_else(|| item.get("command"))
                            .and_then(|v| v.as_str())
                            .unwrap_or(itype)
                            .to_string();
                        *step_counter += 1;
                        let step = *step_counter;
                        if !id.is_empty() {
                            item_steps.insert(id, (step, name.clone()));
                        }
                        on_event(make_tool_event(
                            &req.session_id,
                            step,
                            &name,
                            "running",
                            &format_readable_tool(&name, ""),
                        ));
                    }
                }
            }
            "item.completed" => {
                if let Some(item) = parsed.get("item") {
                    let itype = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if itype == "mcp_tool_call" || itype == "command_execution" {
                        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let is_err = item.get("status").and_then(|v| v.as_str()) == Some("failed")
                            || item.get("error").is_some();
                        if let Some((step, name)) = item_steps.get(id) {
                            on_event(make_tool_event(
                                &req.session_id,
                                *step,
                                name,
                                if is_err { "error" } else { "done" },
                                &format_readable_tool(name, ""),
                            ));
                        }
                    } else if itype == "agent_message" {
                        if let Some(txt) = item.get("text").and_then(|v| v.as_str()) {
                            if !accumulated.is_empty() {
                                accumulated.push_str("\n\n");
                            }
                            accumulated.push_str(txt);
                            on_event(make_delta_event(&req.session_id, txt));
                        }
                    }
                }
            }
            "turn.completed" => {
                *saw_result = true;
                on_event(make_done_event(&req.session_id, accumulated.clone()));
            }
            "turn.failed" | "error" => {
                *saw_result = true;
                let msg = parsed
                    .pointer("/error/message")
                    .or_else(|| parsed.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Codex 执行失败")
                    .to_string();
                on_event(make_error_event(&req.session_id, msg));
            }
            _ => {}
        }
    }
}

fn spawn_agy(
    req: &SendAgentTurnPayload,
    full_prompt: &str,
    conversation_id: Option<&str>,
) -> Result<(Child, String), String> {
    let bin = req
        .cli_path
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(resolve_agy_bin);
    let mut cmd = Command::new(&bin);
    cmd.arg("--output-format")
        .arg("stream-json")
        .arg("--dangerously-skip-permissions")
        .arg("--add-dir")
        .arg(&req.root_path);
    if let Some(m) = req.model.as_deref().filter(|s| !s.trim().is_empty()) {
        cmd.arg("--model").arg(m.trim());
    }
    if let Some(cid) = conversation_id.filter(|s| !s.trim().is_empty()) {
        cmd.arg("--conversation").arg(cid);
    }
    cmd.arg("-p").arg(full_prompt);
    cmd.current_dir(&req.root_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let child = cmd
        .spawn()
        .map_err(|e| format!("启动 AGY 失败 ({bin}): {e}"))?;
    Ok((child, bin))
}

fn spawn_claude(
    req: &SendAgentTurnPayload,
    full_prompt: &str,
    mcp_name: &str,
    mcp_url: &str,
    conversation_id: Option<&str>,
) -> Result<(Child, String), String> {
    let bin = req
        .cli_path
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(resolve_claude_bin);
    let mcp_cfg = json!({
        "mcpServers": {
            mcp_name: {
                "type": "http",
                "url": mcp_url
            }
        }
    })
    .to_string();

    let mut cmd = Command::new(&bin);
    cmd.arg("-p")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--include-partial-messages")
        .arg("--dangerously-skip-permissions")
        .arg("--mcp-config")
        .arg(&mcp_cfg)
        .arg("--add-dir")
        .arg(&req.root_path);
    if let Some(m) = req.model.as_deref().filter(|s| !s.trim().is_empty()) {
        cmd.arg("--model").arg(m.trim());
    }
    if let Some(cid) = conversation_id.filter(|s| !s.trim().is_empty()) {
        cmd.arg("--resume").arg(cid);
    }
    cmd.arg("--").arg(full_prompt);
    cmd.current_dir(&req.root_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let child = cmd
        .spawn()
        .map_err(|e| format!("启动 Claude Code 失败 ({bin}): {e}"))?;
    Ok((child, bin))
}

async fn spawn_codex(
    req: &SendAgentTurnPayload,
    full_prompt: &str,
    mcp_name: &str,
    mcp_url: &str,
    conversation_id: Option<&str>,
) -> Result<(Child, String), String> {
    let mcp_override = format!("mcp_servers.{mcp_name}.url=\"{mcp_url}\"");
    let codex_bin = req
        .cli_path
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(resolve_codex_bin);

    #[cfg(target_os = "windows")]
    let (bin, mut cmd) = {
        let mut c = Command::new("cmd.exe");
        c.arg("/C").arg(&codex_bin);
        c.creation_flags(CREATE_NO_WINDOW);
        (codex_bin.clone(), c)
    };
    #[cfg(not(target_os = "windows"))]
    let (bin, mut cmd) = {
        let c = Command::new(&codex_bin);
        (codex_bin.clone(), c)
    };

    if let Some(cid) = conversation_id.filter(|s| !s.trim().is_empty()) {
        cmd.arg("exec")
            .arg("resume")
            .arg("--json")
            .arg("--skip-git-repo-check")
            .arg("--dangerously-bypass-approvals-and-sandbox")
            .arg("-c")
            .arg(&mcp_override);
        if let Some(m) = req.model.as_deref().filter(|s| !s.trim().is_empty()) {
            cmd.arg("--model").arg(m.trim());
        }
        cmd.arg(cid).arg("-");
    } else {
        cmd.arg("exec")
            .arg("--json")
            .arg("--skip-git-repo-check")
            .arg("--dangerously-bypass-approvals-and-sandbox")
            .arg("-C")
            .arg(&req.root_path)
            .arg("-c")
            .arg(&mcp_override);
        if let Some(m) = req.model.as_deref().filter(|s| !s.trim().is_empty()) {
            cmd.arg("--model").arg(m.trim());
        }
        cmd.arg("-");
    }

    cmd.current_dir(&req.root_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 Codex 失败 ({bin}): {e}"))?;

    // Write prompt to stdin and close immediately so codex exec receives EOF cleanly
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(full_prompt.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    Ok((child, bin))
}

pub fn resolve_claude_bin() -> String {
    if let Ok(custom) = std::env::var("CLAUDE_BIN") {
        if !custom.trim().is_empty() {
            return custom;
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let p = Path::new(&profile).join(".local").join("bin").join("claude.exe");
            if p.exists() {
                return p.to_string_lossy().to_string();
            }
        }
        "claude.exe".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        "claude".to_string()
    }
}

pub fn resolve_codex_bin() -> String {
    if let Ok(custom) = std::env::var("CODEX_BIN") {
        if !custom.trim().is_empty() {
            return custom;
        }
    }
    "codex".to_string()
}

fn make_status_event(session_id: &str, text: &str) -> AgentStreamEvent {
    AgentStreamEvent {
        session_id: session_id.to_string(),
        kind: "status".to_string(),
        conversation_id: None,
        step_index: None,
        tool_name: None,
        tool_state: None,
        tool_detail: None,
        duration_sec: None,
        text: Some(text.to_string()),
        error: None,
    }
}

fn make_init_event(session_id: &str, cid: Option<String>) -> AgentStreamEvent {
    AgentStreamEvent {
        session_id: session_id.to_string(),
        kind: "init".to_string(),
        conversation_id: cid,
        step_index: None,
        tool_name: None,
        tool_state: None,
        tool_detail: None,
        duration_sec: None,
        text: None,
        error: None,
    }
}

fn make_delta_event(session_id: &str, text: &str) -> AgentStreamEvent {
    AgentStreamEvent {
        session_id: session_id.to_string(),
        kind: "delta".to_string(),
        conversation_id: None,
        step_index: None,
        tool_name: None,
        tool_state: None,
        tool_detail: None,
        duration_sec: None,
        text: Some(text.to_string()),
        error: None,
    }
}

fn make_tool_event(
    session_id: &str,
    step: i64,
    name: &str,
    state: &str,
    detail: &str,
) -> AgentStreamEvent {
    AgentStreamEvent {
        session_id: session_id.to_string(),
        kind: "tool".to_string(),
        conversation_id: None,
        step_index: Some(step),
        tool_name: Some(name.to_string()),
        tool_state: Some(state.to_string()),
        tool_detail: Some(detail.to_string()),
        duration_sec: None,
        text: None,
        error: None,
    }
}

fn make_done_event(session_id: &str, text: String) -> AgentStreamEvent {
    AgentStreamEvent {
        session_id: session_id.to_string(),
        kind: "done".to_string(),
        conversation_id: None,
        step_index: None,
        tool_name: None,
        tool_state: None,
        tool_detail: None,
        duration_sec: None,
        text: Some(text),
        error: None,
    }
}

fn make_error_event(session_id: &str, error: String) -> AgentStreamEvent {
    AgentStreamEvent {
        session_id: session_id.to_string(),
        kind: "error".to_string(),
        conversation_id: None,
        step_index: None,
        tool_name: None,
        tool_state: None,
        tool_detail: None,
        duration_sec: None,
        text: None,
        error: Some(error),
    }
}

async fn kill_pid(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("taskkill");
        cmd.args(["/PID", &pid.to_string(), "/T", "/F"]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd.output().await;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
            .await;
    }
}

fn is_internal_mcp_schema_read(name: &str, detail: &str) -> bool {
    let lower = detail.to_lowercase();
    (name == "read_file" || name == "view_file")
        && (lower.contains("mark5-doc")
            || lower.contains("doc_create_skeleton.json")
            || lower.contains("doc_replace_block.json")
            || lower.contains("doc_read_blocks.json"))
}

fn format_readable_tool(name: &str, detail: &str) -> String {
    let lower_name = name.to_lowercase();
    if lower_name.contains("doc_create_skeleton") || detail.contains("doc_create_skeleton") {
        return "创建文档骨架".to_string();
    }
    if lower_name.contains("doc_read_blocks") || detail.contains("doc_read_blocks") {
        return "读取最新文档段落".to_string();
    }
    if lower_name.contains("doc_replace_block") || detail.contains("doc_replace_block") {
        return "修改文档段落".to_string();
    }
    if lower_name.contains("doc_insert_block") || detail.contains("doc_insert_block") {
        return "插入新段落".to_string();
    }
    if lower_name.contains("doc_delete_block") || detail.contains("doc_delete_block") {
        return "删除文档段落".to_string();
    }
    if lower_name.contains("doc_reply_comment") || detail.contains("doc_reply_comment") {
        return "在批注线程中回复".to_string();
    }
    if lower_name.contains("doc_create_comment") || detail.contains("doc_create_comment") {
        return "添加待核实批注".to_string();
    }
    if !detail.trim().is_empty() {
        return detail.trim().chars().take(60).collect();
    }
    name.to_string()
}

const SYSTEM_WRITING_RULES: &str = r#"## 通用写作准则（软件默认）
- 先定主张再动笔：未明确一句话核心观点和主线前，不要直接生成大段成稿；拒绝空洞套话与 AI 预制菜。
- 主线唯一：全文围绕一条主线推进，章标题连起来构成清晰的逻辑链；无关材料再好也删。
- 去除典型 AI 味：
  - 禁用廉价句式与过渡词：不是……而是……、从……到……、与其说……不如说……、不能简单概括成……、值得一提的是、在当今时代背景下，以及生硬凑数的首先、其次、最后。
  - 不做稻草人论证与防御性写作：不虚构“很多人以为……其实不然”式的假想敌；只在事实或材料中真实存在对立时才做对比，不脸谱化任何一方；只在边界条件真正改变结论时才加限定，且紧挨对应论断写，不反复插免责声明。
  - 拒绝表演与凑数：删掉硬凑的金句、强行拔高升华、无意义的三段排比和情绪煽动。
- 只讲透一次：同一观点只在最合适的位置讲透，其余处最多一句呼应；删除紧跟在后的同义复述句。
- 去掉路标句：不用本章将讲……、下面我们来看……、综上所述，让标题承担导航。
- 句式、比喻与阅读摩擦：以短句和中句为主，仅在因果链需要连贯时用长句；比喻只用来解释机制，不好懂或纯装饰的直接删；复用前文概念时当场一句话点明，不让读者回翻；链接包在文字中，不裸贴 URL。"#;

fn build_prompt(req: &SendAgentTurnPayload, mcp_name: &str) -> String {
    let mut parts = vec![
        format!("你是 Mark 5 本地 Markdown 协同写作助手。你的本地文档 MCP 工具集名称为 `{mcp_name}`。"),
        format!(
            "## 工具使用硬性约束（最高优先级）\n- 编辑、新增或删除 Markdown（.md）文档内容时，必须使用 `{mcp_name}` 的 `doc_*` 工具（`doc_create_skeleton` / `doc_read_blocks` / `doc_replace_block` / `doc_insert_block` / `doc_delete_block`），禁止使用文件编辑工具或命令行直接改写 `.md` 文件，否则编辑器无法同步实时动画与 Diff 审阅。\n- 修改或插入段落前，先调用 `doc_read_blocks` 获取最新实时段落与块索引；回复批注用 `doc_reply_comment`，需要人工确认处可用 `doc_create_comment` 挂载批注。"
        ),
        format!("当前工作区：{}", req.root_path),
    ];
    if let Some(ref doc) = req.doc_rel_path {
        parts.push(format!("当前打开文档：{doc}"));
    }
    parts.push(SYSTEM_WRITING_RULES.to_string());
    if let Some(ref personal) = req.personal_writing_prompt {
        let trimmed = personal.trim();
        if !trimmed.is_empty() {
            parts.push(format!("## 个人写作偏好（用户设定）\n{trimmed}"));
        }
    }

    if let Some(ref md) = req.current_markdown {
        if !md.trim().is_empty() {
            let truncated: String = md.chars().take(6000).collect();
            parts.push(format!("当前文档内容片段：\n```markdown\n{truncated}\n```"));
        }
    }
    if let Some(ref tc) = req.thread_context {
        parts.push(format!(
            "用户针对第 {} 段引用“{}”发起批注（threadId={}）：{}",
            tc.block_index, tc.quote, tc.thread_id, req.prompt
        ));
    } else {
        parts.push(format!("用户指令：{}", req.prompt));
    }
    parts.join("\n\n")
}
