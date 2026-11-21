use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::RwLock;

use crate::workspace::{split_markdown_blocks, WorkspaceManager};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone)]
pub struct ActiveSessionContext {
    pub session_id: String,
    pub root_path: String,
    pub doc_rel_path: Option<String>,
}

pub type DocOpCallback = Arc<dyn Fn(Value) + Send + Sync>;

pub struct DocMcpServer {
    pub server_name: String,
    workspace: Arc<WorkspaceManager>,
    on_doc_op: DocOpCallback,
    active_ctx: RwLock<Option<ActiveSessionContext>>,
    port: RwLock<u16>,
    registered: RwLock<bool>,
}

impl DocMcpServer {
    pub fn new(workspace: Arc<WorkspaceManager>, on_doc_op: DocOpCallback) -> Arc<Self> {
        let pid = std::process::id();
        Arc::new(Self {
            server_name: format!("mark5-doc-{pid}"),
            workspace,
            on_doc_op,
            active_ctx: RwLock::new(None),
            port: RwLock::new(0),
            registered: RwLock::new(false),
        })
    }

    pub async fn set_active_context(&self, ctx: ActiveSessionContext) {
        let mut lock = self.active_ctx.write().await;
        *lock = Some(ctx);
    }

    pub fn stop_sync(&self) {
        let bin = resolve_agy_bin();
        let mut cmd = std::process::Command::new(bin);
        cmd.args(["mcp", "remove", &self.server_name]);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let _ = cmd.output();
    }

    pub async fn start(self: &Arc<Self>) -> Result<(), String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("MCP bind failed: {e}"))?;
        let local_port = listener
            .local_addr()
            .map_err(|e| format!("MCP local_addr failed: {e}"))?
            .port();
        *self.port.write().await = local_port;

        let app = Router::new()
            .route("/mcp", post(handle_mcp_post).get(handle_mcp_get))
            .with_state(Arc::clone(self));

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        // Register ephemeral per-instance MCP server with agy CLI (multi-instance safe!)
        let url = format!("http://127.0.0.1:{local_port}/mcp");
        let _ = self.run_agy_mcp(&["mcp", "remove", &self.server_name]).await;
        if self
            .run_agy_mcp(&["mcp", "add", "--type", "http", &self.server_name, &url])
            .await
        {
            *self.registered.write().await = true;
        }
        Ok(())
    }

    async fn run_agy_mcp(&self, args: &[&str]) -> bool {
        let bin = resolve_agy_bin();
        let mut cmd = Command::new(bin);
        cmd.args(args);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        match cmd.output().await {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    async fn handle_tool_call(&self, name: &str, args: &Value) -> Result<Value, String> {
        let ctx_opt = self.active_ctx.read().await.clone();
        let ctx = ctx_opt.ok_or_else(|| "No active session context".to_string())?;
        let root_path = args
            .get("rootPath")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(&ctx.root_path)
            .to_string();

        match name {
            "doc_create_skeleton" => {
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("未命名文档");
                let filename = args
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| {
                        let s = s.trim();
                        if s.ends_with(".md") {
                            s.to_string()
                        } else {
                            format!("{s}.md")
                        }
                    })
                    .unwrap_or_else(|| {
                        let safe: String = title
                            .chars()
                            .filter(|c| !['\\', '/', ':', '*', '?', '"', '<', '>', '|'].contains(c))
                            .collect();
                        let safe = safe.trim();
                        if safe.is_empty() {
                            format!("doc-{}.md", chrono::Utc::now().timestamp())
                        } else {
                            format!("{safe}.md")
                        }
                    });

                let author = args.get("author").and_then(|v| v.as_str()).unwrap_or("你");
                let date = chrono::Local::now().format("%Y-%m-%d").to_string();
                let mut lines = vec![
                    format!("# {title}"),
                    String::new(),
                    format!("> {date} · {author}"),
                    String::new(),
                ];
                if let Some(sections) = args.get("sections").and_then(|v| v.as_array()) {
                    for sec in sections {
                        let h = sec.get("heading").and_then(|v| v.as_str()).unwrap_or("章节");
                        let ph = sec
                            .get("placeholder")
                            .and_then(|v| v.as_str())
                            .unwrap_or("待撰写正文...");
                        lines.push(format!("## {h}"));
                        lines.push(String::new());
                        lines.push(format!("> [待撰写占位块] {ph}"));
                        lines.push(String::new());
                    }
                }
                let md = lines.join("\n").trim_end().to_string() + "\n";
                self.workspace
                    .write_markdown_file(&root_path, &filename, &md)
                    .await?;
                let _ = self
                    .workspace
                    .append_history(&root_path, &filename, "agent", &format!("创建骨架：{title}"), &md)
                    .await;

                // Update active context docRelPath
                {
                    let mut lock = self.active_ctx.write().await;
                    if let Some(c) = lock.as_mut() {
                        c.doc_rel_path = Some(filename.clone());
                    }
                }

                (self.on_doc_op)(json!({
                    "type": "create_skeleton",
                    "sessionId": ctx.session_id,
                    "docRelPath": filename,
                    "title": title,
                    "markdown": md
                }));

                let blocks = split_markdown_blocks(&md);
                Ok(json!({
                    "ok": true,
                    "docRelPath": filename,
                    "blockCount": blocks.len(),
                    "blocks": blocks.iter().enumerate().map(|(i, b)| json!({"index": i, "markdown": b})).collect::<Vec<_>>()
                }))
            }

            "doc_read_blocks" => {
                let doc_rel = args
                    .get("docRelPath")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or(ctx.doc_rel_path)
                    .ok_or_else(|| "Missing docRelPath".to_string())?;
                let md = self.workspace.get_latest_markdown(&root_path, &doc_rel).await;
                let blocks = split_markdown_blocks(&md);
                Ok(json!({
                    "docRelPath": doc_rel,
                    "blockCount": blocks.len(),
                    "blocks": blocks.iter().enumerate().map(|(i, b)| json!({"index": i, "markdown": b})).collect::<Vec<_>>()
                }))
            }

            "doc_replace_block" => {
                let doc_rel = args
                    .get("docRelPath")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or(ctx.doc_rel_path)
                    .ok_or_else(|| "Missing docRelPath".to_string())?;
                let block_index = args.get("blockIndex").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let new_md = args.get("newMarkdown").and_then(|v| v.as_str()).unwrap_or("");
                let label = args
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("更新段落");

                let old_md = self.workspace.get_latest_markdown(&root_path, &doc_rel).await;
                let _ = self
                    .workspace
                    .append_history(&root_path, &doc_rel, "agent", &format!("修改前自动备份：{label}"), &old_md)
                    .await;

                let mut blocks = split_markdown_blocks(&old_md);
                if block_index < blocks.len() {
                    blocks[block_index] = new_md.to_string();
                } else {
                    blocks.push(new_md.to_string());
                }
                let updated = blocks.join("\n\n") + "\n";
                self.workspace
                    .write_markdown_file(&root_path, &doc_rel, &updated)
                    .await?;

                (self.on_doc_op)(json!({
                    "type": "replace_block",
                    "sessionId": ctx.session_id,
                    "docRelPath": doc_rel,
                    "blockIndex": block_index,
                    "newMarkdown": new_md,
                    "label": label,
                    "resultingMarkdown": updated
                }));

                Ok(json!({ "ok": true, "docRelPath": doc_rel, "blockIndex": block_index, "blockCount": blocks.len() }))
            }

            "doc_insert_block" => {
                let doc_rel = args
                    .get("docRelPath")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or(ctx.doc_rel_path)
                    .ok_or_else(|| "Missing docRelPath".to_string())?;
                let after_index = args.get("afterBlockIndex").and_then(|v| v.as_i64()).unwrap_or(-1);
                let new_md = args.get("newMarkdown").and_then(|v| v.as_str()).unwrap_or("");
                let label = args.get("label").and_then(|v| v.as_str()).unwrap_or("插入段落");

                let old_md = self.workspace.get_latest_markdown(&root_path, &doc_rel).await;
                let _ = self
                    .workspace
                    .append_history(&root_path, &doc_rel, "agent", &format!("插入前自动备份：{label}"), &old_md)
                    .await;

                let mut blocks = split_markdown_blocks(&old_md);
                let insert_at = if after_index < 0 {
                    0
                } else {
                    ((after_index + 1) as usize).min(blocks.len())
                };
                blocks.insert(insert_at, new_md.to_string());
                let updated = blocks.join("\n\n") + "\n";
                self.workspace
                    .write_markdown_file(&root_path, &doc_rel, &updated)
                    .await?;

                (self.on_doc_op)(json!({
                    "type": "insert_block",
                    "sessionId": ctx.session_id,
                    "docRelPath": doc_rel,
                    "afterBlockIndex": after_index,
                    "newMarkdown": new_md,
                    "label": label,
                    "resultingMarkdown": updated
                }));

                Ok(json!({ "ok": true, "docRelPath": doc_rel, "insertedAt": insert_at, "blockCount": blocks.len() }))
            }

            "doc_delete_block" => {
                let doc_rel = args
                    .get("docRelPath")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or(ctx.doc_rel_path)
                    .ok_or_else(|| "Missing docRelPath".to_string())?;
                let block_index = args.get("blockIndex").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let label = args.get("label").and_then(|v| v.as_str()).unwrap_or("删除段落");

                let old_md = self.workspace.get_latest_markdown(&root_path, &doc_rel).await;
                let _ = self
                    .workspace
                    .append_history(&root_path, &doc_rel, "agent", &format!("删除前自动备份：{label}"), &old_md)
                    .await;

                let mut blocks = split_markdown_blocks(&old_md);
                if block_index < blocks.len() {
                    blocks.remove(block_index);
                }
                let updated = blocks.join("\n\n") + "\n";
                self.workspace
                    .write_markdown_file(&root_path, &doc_rel, &updated)
                    .await?;

                (self.on_doc_op)(json!({
                    "type": "delete_block",
                    "sessionId": ctx.session_id,
                    "docRelPath": doc_rel,
                    "blockIndex": block_index,
                    "label": label,
                    "resultingMarkdown": updated
                }));

                Ok(json!({ "ok": true, "docRelPath": doc_rel, "blockCount": blocks.len() }))
            }

            "doc_reply_comment" => {
                let doc_rel = args
                    .get("docRelPath")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or(ctx.doc_rel_path)
                    .ok_or_else(|| "Missing docRelPath".to_string())?;
                let thread_id = args.get("threadId").and_then(|v| v.as_str()).unwrap_or("");
                let reply_text = args.get("replyText").and_then(|v| v.as_str()).unwrap_or("");

                (self.on_doc_op)(json!({
                    "type": "reply_comment",
                    "sessionId": ctx.session_id,
                    "docRelPath": doc_rel,
                    "threadId": thread_id,
                    "replyText": reply_text
                }));
                Ok(json!({ "ok": true }))
            }

            "doc_create_comment" => {
                let doc_rel = args
                    .get("docRelPath")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or(ctx.doc_rel_path)
                    .ok_or_else(|| "Missing docRelPath".to_string())?;
                let block_index = args.get("blockIndex").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let quote = args.get("quote").and_then(|v| v.as_str()).unwrap_or("");
                let question_text = args.get("questionText").and_then(|v| v.as_str()).unwrap_or("");

                (self.on_doc_op)(json!({
                    "type": "create_comment",
                    "sessionId": ctx.session_id,
                    "docRelPath": doc_rel,
                    "blockIndex": block_index,
                    "quote": quote,
                    "questionText": question_text
                }));
                Ok(json!({ "ok": true }))
            }

            "doc_read_history" => {
                let doc_rel = args
                    .get("docRelPath")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or(ctx.doc_rel_path)
                    .ok_or_else(|| "Missing docRelPath".to_string())?;
                let list = self.workspace.list_history(&root_path, &doc_rel).await;
                Ok(json!({ "docRelPath": doc_rel, "snapshots": list }))
            }

            _ => Err(format!("Unknown MCP tool: {name}")),
        }
    }
}

async fn handle_mcp_get() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "ok": true, "protocol": "mcp-http" })))
}

async fn handle_mcp_post(
    State(server): State<Arc<DocMcpServer>>,
    body: String,
) -> impl IntoResponse {
    if body.trim().is_empty() {
        return (StatusCode::ACCEPTED, Json(json!({}))).into_response();
    }
    let req: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return (StatusCode::ACCEPTED, Json(json!({}))).into_response(),
    };
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");

    if method.starts_with("notifications/") || id.is_none() {
        return (StatusCode::ACCEPTED, Json(json!({}))).into_response();
    }

    let id_val = id.unwrap_or(Value::Null);
    let resp = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": server.server_name, "version": "0.1.0" }
            }
        }),
        "server/discover" | "ping" => json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "result": { "ok": true }
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "result": { "tools": tool_definitions() }
        }),
        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match server.handle_tool_call(tool_name, &args).await {
                Ok(res) => json!({
                    "jsonrpc": "2.0",
                    "id": id_val,
                    "result": {
                        "content": [{ "type": "text", "text": serde_json::to_string(&res).unwrap_or_default() }]
                    }
                }),
                Err(err) => json!({
                    "jsonrpc": "2.0",
                    "id": id_val,
                    "result": {
                        "isError": true,
                        "content": [{ "type": "text", "text": err }]
                    }
                }),
            }
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "result": {}
        }),
    };

    (StatusCode::OK, Json(resp)).into_response()
}

pub fn resolve_agy_bin() -> String {
    if let Ok(custom) = std::env::var("AGY_BIN") {
        if !custom.trim().is_empty() {
            return custom;
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = Path::new(&local).join("agy").join("bin").join("agy.exe");
            if p.exists() {
                return p.to_string_lossy().to_string();
            }
        }
        "agy.exe".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        "agy".to_string()
    }
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "doc_create_skeleton",
            "description": "创建新的 Markdown 文档骨架并在 Mark 5 编辑器中打开",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "filename": { "type": "string" },
                    "author": { "type": "string" },
                    "sections": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "heading": { "type": "string" },
                                "placeholder": { "type": "string" }
                            },
                            "required": ["heading"]
                        }
                    }
                },
                "required": ["title", "sections"]
            }
        }),
        json!({
            "name": "doc_read_blocks",
            "description": "读取当前 Markdown 文档按空行切分的最新段落块列表（含人工刚修改的内容）",
            "inputSchema": {
                "type": "object",
                "properties": { "docRelPath": { "type": "string" } }
            }
        }),
        json!({
            "name": "doc_replace_block",
            "description": "替换指定索引的段落块并在编辑器中播放 Agent 光标修改动画",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "docRelPath": { "type": "string" },
                    "blockIndex": { "type": "integer" },
                    "newMarkdown": { "type": "string" },
                    "label": { "type": "string" }
                },
                "required": ["blockIndex", "newMarkdown"]
            }
        }),
        json!({
            "name": "doc_insert_block",
            "description": "在指定段落索引之后插入新段落块",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "docRelPath": { "type": "string" },
                    "afterBlockIndex": { "type": "integer" },
                    "newMarkdown": { "type": "string" },
                    "label": { "type": "string" }
                },
                "required": ["afterBlockIndex", "newMarkdown"]
            }
        }),
        json!({
            "name": "doc_delete_block",
            "description": "删除指定索引的段落块",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "docRelPath": { "type": "string" },
                    "blockIndex": { "type": "integer" },
                    "label": { "type": "string" }
                },
                "required": ["blockIndex"]
            }
        }),
        json!({
            "name": "doc_reply_comment",
            "description": "在指定批注线程中直接回复用户",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "docRelPath": { "type": "string" },
                    "threadId": { "type": "string" },
                    "replyText": { "type": "string" }
                },
                "required": ["threadId", "replyText"]
            }
        }),
        json!({
            "name": "doc_create_comment",
            "description": "对文档中某个需要人工核实或确认的段落主动挂载批注提问",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "docRelPath": { "type": "string" },
                    "blockIndex": { "type": "integer" },
                    "quote": { "type": "string" },
                    "questionText": { "type": "string" }
                },
                "required": ["blockIndex", "quote", "questionText"]
            }
        }),
        json!({
            "name": "doc_read_history",
            "description": "读取指定文档的版本历史快照列表",
            "inputSchema": {
                "type": "object",
                "properties": { "docRelPath": { "type": "string" } }
            }
        }),
    ]
}

// Keep HashMap import clean
#[allow(dead_code)]
type _Unused = HashMap<String, String>;
