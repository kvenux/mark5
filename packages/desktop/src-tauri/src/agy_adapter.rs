use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
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

        let full_prompt = build_prompt(&req, &self.mcp.server_name);
        let bin = resolve_agy_bin();
        let mut cmd = Command::new(&bin);
        cmd.arg("--output-format")
            .arg("stream-json")
            .arg("--dangerously-skip-permissions")
            .arg("--add-dir")
            .arg(&req.root_path);
        if let Some(ref cid) = conversation_id {
            if !cid.trim().is_empty() {
                cmd.arg("--conversation").arg(cid);
            }
        }
        cmd.arg("-p").arg(&full_prompt);
        cmd.current_dir(&req.root_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        on_event(AgentStreamEvent {
            session_id: req.session_id.clone(),
            kind: "status".to_string(),
            conversation_id: None,
            step_index: None,
            tool_name: None,
            tool_state: None,
            tool_detail: None,
            duration_sec: None,
            text: Some("正在连接 AGY...".to_string()),
            error: None,
        });

        let mut child: Child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("启动 AGY 失败 ({bin}): {e}");
                on_event(AgentStreamEvent {
                    session_id: req.session_id.clone(),
                    kind: "error".to_string(),
                    conversation_id: None,
                    step_index: None,
                    tool_name: None,
                    tool_state: None,
                    tool_detail: None,
                    duration_sec: None,
                    text: None,
                    error: Some(msg.clone()),
                });
                return Err(msg);
            }
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
                let ev_name = parsed.get("event").and_then(|v| v.as_str()).unwrap_or("");
                match ev_name {
                    "init" => {
                        let cid = parsed
                            .get("conversation_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        if let Some(ref c) = cid {
                            let _ = self
                                .workspace
                                .update_session_conversation_id(&req.root_path, &req.session_id, c)
                                .await;
                        }
                        on_event(AgentStreamEvent {
                            session_id: req.session_id.clone(),
                            kind: "init".to_string(),
                            conversation_id: cid,
                            step_index: None,
                            tool_name: None,
                            tool_state: None,
                            tool_detail: None,
                            duration_sec: None,
                            text: None,
                            error: None,
                        });
                    }
                    "tool" => {
                        let raw_name = parsed
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("tool");
                        let detail_raw = parsed
                            .get("detail")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if is_internal_mcp_schema_read(raw_name, detail_raw) {
                            continue;
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
                            on_event(AgentStreamEvent {
                                session_id: req.session_id.clone(),
                                kind: "delta".to_string(),
                                conversation_id: None,
                                step_index: None,
                                tool_name: None,
                                tool_state: None,
                                tool_detail: None,
                                duration_sec: None,
                                text: Some(chunk.to_string()),
                                error: None,
                            });
                        }
                    }
                    "result" => {
                        saw_result = true;
                        let final_text = parsed
                            .get("result")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&accumulated)
                            .to_string();
                        on_event(AgentStreamEvent {
                            session_id: req.session_id.clone(),
                            kind: "done".to_string(),
                            conversation_id: None,
                            step_index: None,
                            tool_name: None,
                            tool_state: None,
                            tool_detail: None,
                            duration_sec: None,
                            text: Some(final_text),
                            error: None,
                        });
                    }
                    "error" => {
                        saw_result = true;
                        let err_msg = parsed
                            .get("error")
                            .or_else(|| parsed.get("message"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("AGY 执行失败")
                            .to_string();
                        on_event(AgentStreamEvent {
                            session_id: req.session_id.clone(),
                            kind: "error".to_string(),
                            conversation_id: None,
                            step_index: None,
                            tool_name: None,
                            tool_state: None,
                            tool_detail: None,
                            duration_sec: None,
                            text: None,
                            error: Some(err_msg),
                        });
                    }
                    _ => {}
                }
            }
        }

        let status = child.wait().await;
        let stderr_buf = stderr_handle.await.unwrap_or_default();

        // Guard stale child: only clean up if our PID is still the active one
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
                on_event(AgentStreamEvent {
                    session_id: req.session_id.clone(),
                    kind: "done".to_string(),
                    conversation_id: None,
                    step_index: None,
                    tool_name: None,
                    tool_state: None,
                    tool_detail: None,
                    duration_sec: None,
                    text: Some(accumulated),
                    error: None,
                });
            } else {
                let err = if stderr_buf.trim().is_empty() {
                    format!("AGY 进程异常退出 (code={code})")
                } else {
                    stderr_buf.trim().to_string()
                };
                on_event(AgentStreamEvent {
                    session_id: req.session_id.clone(),
                    kind: "error".to_string(),
                    conversation_id: None,
                    step_index: None,
                    tool_name: None,
                    tool_state: None,
                    tool_detail: None,
                    duration_sec: None,
                    text: None,
                    error: Some(err),
                });
            }
        }

        Ok(())
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
        && (lower.contains("mark5-doc") || lower.contains("doc_create_skeleton.json") || lower.contains("doc_replace_block.json") || lower.contains("doc_read_blocks.json"))
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

fn build_prompt(req: &SendAgentTurnPayload, mcp_name: &str) -> String {
    let mut parts = vec![
        format!("你是 Mark 5 本地 Markdown 协同写作助手。你的本地文档 MCP 工具集名称为 `{mcp_name}`。"),
        format!("当前工作区：{}", req.root_path),
    ];
    if let Some(ref doc) = req.doc_rel_path {
        parts.push(format!("当前打开文档：{doc}"));
    }
    parts.push("协作规范：创建新文档用 doc_create_skeleton；修改段落前先用 doc_read_blocks 获取最新内容再用 doc_replace_block / doc_insert_block；批注问题用 doc_reply_comment 回复。".to_string());

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
