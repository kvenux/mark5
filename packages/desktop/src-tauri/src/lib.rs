mod agy_adapter;
mod mcp_server;
mod types;
mod workspace;

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::agy_adapter::{resolve_claude_bin, resolve_codex_bin, AgyAdapter};
use crate::mcp_server::{resolve_agy_bin, DocMcpServer};
use crate::types::{
    CliStatusItem, CommentThread, CreateSessionPayload, ExportDocPayload, ExportDocResult,
    FileEntry, HistorySnapshot, ModelOption, SaveHistoryPayload, SendAgentTurnPayload,
    SessionRecord, WorkspaceInfo,
};
use crate::workspace::WorkspaceManager;

struct AppState {
    workspace: Arc<WorkspaceManager>,
    mcp: Arc<DocMcpServer>,
    agy: Arc<AgyAdapter>,
    win_counter: AtomicUsize,
}

fn make_workspace_info(root_path: &str) -> WorkspaceInfo {
    let name = Path::new(root_path)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(root_path)
        .to_string();
    WorkspaceInfo {
        id: format!("ws-{}", Uuid::new_v5(&Uuid::NAMESPACE_OID, root_path.as_bytes()).simple()),
        name,
        root_path: root_path.to_string(),
    }
}

fn app_state_file() -> Option<std::path::PathBuf> {
    user_home_dir().map(|h| h.join(".mark5").join("app-state.json"))
}

async fn read_last_workspace() -> Option<String> {
    let path = app_state_file()?;
    let raw = tokio::fs::read_to_string(&path).await.ok()?;
    let val: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let last = val.get("lastWorkspace").and_then(|v| v.as_str())?.trim();
    if !last.is_empty() && Path::new(last).is_dir() {
        Some(last.to_string())
    } else {
        None
    }
}

fn read_app_state_sync() -> serde_json::Value {
    let Some(path) = app_state_file() else {
        return serde_json::json!({});
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn read_saved_theme_sync() -> String {
    read_app_state_sync()
        .get("theme")
        .and_then(|v| v.as_str())
        .unwrap_or("warm")
        .to_string()
}

fn theme_bg_color(theme: &str) -> tauri::window::Color {
    if theme == "dark" {
        tauri::window::Color(0x14, 0x14, 0x16, 0xff)
    } else {
        tauri::window::Color(0xff, 0xff, 0xff, 0xff)
    }
}

fn theme_native_opt(theme: &str) -> Option<tauri::Theme> {
    if theme == "dark" {
        Some(tauri::Theme::Dark)
    } else {
        Some(tauri::Theme::Light)
    }
}

async fn save_last_workspace(root_path: &str) {
    let Some(path) = app_state_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let mut payload = read_app_state_sync();
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("lastWorkspace".to_string(), serde_json::json!(root_path));
    } else {
        payload = serde_json::json!({ "lastWorkspace": root_path });
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(&payload) {
        let _ = tokio::fs::write(&path, bytes).await;
    }
}

#[tauri::command]
async fn set_app_theme(app: AppHandle, theme: String) -> Result<(), String> {
    let clean = match theme.trim() {
        "dark" => "dark",
        "light" => "light",
        _ => "warm",
    };
    if let Some(path) = app_state_file() {
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let mut payload = read_app_state_sync();
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("theme".to_string(), serde_json::json!(clean));
        } else {
            payload = serde_json::json!({ "theme": clean });
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&payload) {
            let _ = tokio::fs::write(&path, bytes).await;
        }
    }
    let bg = theme_bg_color(clean);
    let native = theme_native_opt(clean);
    for w in app.webview_windows().values() {
        let _ = w.set_background_color(Some(bg));
        let _ = w.set_theme(native);
    }
    Ok(())
}

#[tauri::command]
async fn get_default_workspace() -> Result<Option<WorkspaceInfo>, String> {
    if let Ok(env_ws) = std::env::var("MARK5_WORKSPACE") {
        let trimmed = env_ws.trim();
        if !trimmed.is_empty() {
            return Ok(Some(make_workspace_info(trimmed)));
        }
    }

    if let Some(last) = read_last_workspace().await {
        return Ok(Some(make_workspace_info(&last)));
    }

    // First launch (no saved workspace yet): let frontend show the clean Welcome Launcher
    Ok(None)
}

#[tauri::command]
async fn pick_workspace_folder(app: AppHandle) -> Result<Option<WorkspaceInfo>, String> {
    let picked = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("选择工作区目录")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| e.to_string())?;

    if let Some(fp) = picked {
        let root = fp.to_string();
        save_last_workspace(&root).await;
        Ok(Some(make_workspace_info(&root)))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn list_workspace_files(
    state: State<'_, AppState>,
    root_path: String,
) -> Result<Vec<FileEntry>, String> {
    Ok(state.workspace.list_markdown_files(&root_path).await)
}

#[tauri::command]
async fn list_workspace_all_files(
    state: State<'_, AppState>,
    root_path: String,
) -> Result<Vec<FileEntry>, String> {
    Ok(state.workspace.list_all_files(&root_path).await)
}

#[tauri::command]
async fn read_markdown_file(
    state: State<'_, AppState>,
    root_path: String,
    rel_path: String,
) -> Result<String, String> {
    Ok(state.workspace.read_markdown_file(&root_path, &rel_path).await)
}

#[tauri::command]
async fn write_markdown_file(
    state: State<'_, AppState>,
    root_path: String,
    rel_path: String,
    content: String,
) -> Result<(), String> {
    state
        .workspace
        .write_markdown_file(&root_path, &rel_path, &content)
        .await
}

#[tauri::command]
async fn rename_entry(
    state: State<'_, AppState>,
    root_path: String,
    old_rel_path: String,
    new_name: String,
) -> Result<String, String> {
    state
        .workspace
        .rename_entry(&root_path, &old_rel_path, &new_name)
        .await
}

#[tauri::command]
async fn duplicate_file(
    state: State<'_, AppState>,
    root_path: String,
    rel_path: String,
) -> Result<String, String> {
    state.workspace.duplicate_file(&root_path, &rel_path).await
}

#[tauri::command]
async fn delete_entry(
    state: State<'_, AppState>,
    root_path: String,
    rel_path: String,
) -> Result<(), String> {
    state.workspace.delete_entry(&root_path, &rel_path).await
}

#[tauri::command]
async fn create_folder(
    state: State<'_, AppState>,
    root_path: String,
    parent_rel_path: String,
    folder_name: String,
) -> Result<String, String> {
    state
        .workspace
        .create_folder(&root_path, &parent_rel_path, &folder_name)
        .await
}

#[tauri::command]
async fn reveal_in_explorer(root_path: String, rel_path: String) -> Result<(), String> {
    let full = Path::new(&root_path).join(&rel_path);
    #[cfg(target_os = "windows")]
    {
        let arg = format!("/select,{}", full.to_string_lossy());
        let _ = std::process::Command::new("explorer").arg(arg).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .args(["-R", &full.to_string_lossy()])
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = full.parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }
    Ok(())
}

#[tauri::command]
async fn sync_live_document(
    state: State<'_, AppState>,
    root_path: String,
    doc_rel_path: String,
    markdown: String,
) -> Result<(), String> {
    state
        .workspace
        .set_live_markdown(&root_path, &doc_rel_path, markdown)
        .await;
    Ok(())
}

#[tauri::command]
async fn list_sessions(
    state: State<'_, AppState>,
    root_path: String,
) -> Result<Vec<SessionRecord>, String> {
    Ok(state.workspace.list_sessions(&root_path).await)
}

#[tauri::command]
async fn create_session(
    state: State<'_, AppState>,
    payload: CreateSessionPayload,
) -> Result<SessionRecord, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let session = SessionRecord {
        id: format!("sess-{}", &Uuid::new_v4().simple().to_string()[..10]),
        workspace_path: payload.root_path.clone(),
        title: payload.title,
        engine: payload.engine,
        model: payload.model,
        conversation_id: None,
        doc_rel_path: payload.doc_rel_path,
        is_artifact: Some(false),
        pinned: Some(false),
        archived: Some(false),
        unread: Some(false),
        created_at: now,
        updated_at: now,
        messages: vec![],
    };
    state
        .workspace
        .save_session(&payload.root_path, session.clone())
        .await?;
    Ok(session)
}

#[tauri::command]
async fn save_session(
    state: State<'_, AppState>,
    root_path: String,
    session: SessionRecord,
) -> Result<(), String> {
    state.workspace.save_session(&root_path, session).await
}

#[tauri::command]
async fn delete_session(
    state: State<'_, AppState>,
    root_path: String,
    session_id: String,
) -> Result<(), String> {
    state.workspace.delete_session(&root_path, &session_id).await
}

#[tauri::command]
async fn list_threads(
    state: State<'_, AppState>,
    root_path: String,
    doc_rel_path: String,
) -> Result<Vec<CommentThread>, String> {
    Ok(state.workspace.list_threads(&root_path, &doc_rel_path).await)
}

#[tauri::command]
async fn save_threads(
    state: State<'_, AppState>,
    root_path: String,
    doc_rel_path: String,
    threads: Vec<CommentThread>,
) -> Result<(), String> {
    state
        .workspace
        .save_threads(&root_path, &doc_rel_path, threads)
        .await
}

#[tauri::command]
async fn list_history(
    state: State<'_, AppState>,
    root_path: String,
    doc_rel_path: String,
) -> Result<Vec<HistorySnapshot>, String> {
    Ok(state.workspace.list_history(&root_path, &doc_rel_path).await)
}

#[tauri::command]
async fn save_history_snapshot(
    state: State<'_, AppState>,
    root_path: String,
    payload: SaveHistoryPayload,
) -> Result<HistorySnapshot, String> {
    state
        .workspace
        .append_history(
            &root_path,
            &payload.doc_rel_path,
            &payload.actor,
            &payload.summary,
            &payload.markdown,
        )
        .await
}

#[tauri::command]
async fn send_agent_turn(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: SendAgentTurnPayload,
) -> Result<(), String> {
    let existing = state
        .workspace
        .get_session(&payload.root_path, &payload.session_id)
        .await;
    let engine = existing
        .as_ref()
        .map(|s| s.engine.clone())
        .unwrap_or_else(|| "agy".to_string());
    let conv_id = existing.and_then(|s| s.conversation_id);
    let app_clone = app.clone();
    let on_event = Arc::new(move |ev: crate::types::AgentStreamEvent| {
        let _ = app_clone.emit("mc:agent-event", ev);
    });
    state.agy.run_turn(payload, engine, conv_id, on_event).await
}

#[tauri::command]
async fn abort_agent_turn(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    state.agy.abort(&session_id).await;
    Ok(())
}

#[tauri::command]
async fn export_document(
    app: AppHandle,
    payload: ExportDocPayload,
) -> Result<ExportDocResult, String> {
    let base_name = Path::new(&payload.doc_rel_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document")
        .to_string();
    let ext = match payload.format.as_str() {
        "html" => "html",
        _ => "md",
    };
    let default_file = format!("{base_name}.{ext}");

    let picked = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("导出文档")
            .set_file_name(&default_file)
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;

    let Some(fp) = picked else {
        return Ok(ExportDocResult {
            ok: false,
            out_path: None,
        });
    };

    let out_path = fp.to_string();
    let content = if payload.format == "html" {
        let body = payload.html.unwrap_or_else(|| payload.markdown.clone());
        format!(
            "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"/><title>{base_name}</title><style>body{{max-width:760px;margin:48px auto;padding:0 24px;font-family:-apple-system,BlinkMacSystemFont,'PingFang SC','Microsoft YaHei',sans-serif;line-height:1.75;color:#1a1918}}pre{{background:#f4f3ee;padding:14px;border-radius:8px;overflow:auto}}code{{font-family:Consolas,monospace}}</style></head><body>{body}</body></html>"
        )
    } else {
        payload.markdown
    };

    tokio::fs::write(&out_path, content.as_bytes())
        .await
        .map_err(|e| format!("export write failed: {e}"))?;

    Ok(ExportDocResult {
        ok: true,
        out_path: Some(out_path),
    })
}

#[tauri::command]
async fn check_cli_status(
    agy_path: Option<String>,
    claude_path: Option<String>,
    codex_path: Option<String>,
) -> Result<Vec<CliStatusItem>, String> {
    let specs = [
        (
            "agy",
            agy_path
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(resolve_agy_bin),
            false,
        ),
        (
            "claude",
            claude_path
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(resolve_claude_bin),
            false,
        ),
        (
            "codex",
            codex_path
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(resolve_codex_bin),
            true,
        ),
    ];

    let mut out = Vec::new();
    for (eng, bin, via_cmd) in specs {
        let mut cmd = if cfg!(target_os = "windows") && via_cmd {
            let mut c = tokio::process::Command::new("cmd.exe");
            c.args(["/C", &bin, "--version"]);
            c
        } else {
            let mut c = tokio::process::Command::new(&bin);
            c.arg("--version");
            c
        };
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);
        match cmd.output().await {
            Ok(res) if res.status.success() => {
                let ver = String::from_utf8_lossy(&res.stdout)
                    .lines()
                    .next()
                    .unwrap_or("已安装")
                    .trim()
                    .to_string();
                out.push(CliStatusItem {
                    engine: eng.to_string(),
                    available: true,
                    version: ver,
                    resolved_path: bin,
                });
            }
            _ => {
                out.push(CliStatusItem {
                    engine: eng.to_string(),
                    available: false,
                    version: "未检测到".to_string(),
                    resolved_path: bin,
                });
            }
        }
    }
    Ok(out)
}

fn user_home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
}

#[tauri::command]
async fn list_engine_models(
    engine: String,
    cli_path: Option<String>,
) -> Result<Vec<ModelOption>, String> {
    let eng = engine.trim().to_lowercase();
    match eng.as_str() {
        "agy" => {
            let bin = cli_path
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(resolve_agy_bin);
            let mut cmd = tokio::process::Command::new(&bin);
            cmd.arg("models");
            #[cfg(target_os = "windows")]
            cmd.creation_flags(0x08000000);
            let mut out = Vec::new();
            if let Ok(res) = cmd.output().await {
                if res.status.success() {
                    let stdout = String::from_utf8_lossy(&res.stdout);
                    for line in stdout.lines() {
                        let trimmed = line.trim();
                        if let Some((id, label)) = trimmed.split_once('\t') {
                            let id_s = id.trim();
                            let label_s = label.trim();
                            if !id_s.is_empty() {
                                out.push(ModelOption {
                                    id: id_s.to_string(),
                                    label: if label_s.is_empty() {
                                        id_s.to_string()
                                    } else {
                                        label_s.to_string()
                                    },
                                });
                            }
                        }
                    }
                }
            }
            Ok(out)
        }
        "codex" => {
            let mut out = Vec::new();
            if let Some(home) = user_home_dir() {
                let cache_path = home.join(".codex").join("models_cache.json");
                if let Ok(raw) = tokio::fs::read_to_string(&cache_path).await {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                        if let Some(arr) = val.get("models").and_then(|v| v.as_array()) {
                            for item in arr {
                                let vis = item
                                    .get("visibility")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("list");
                                if vis == "hide" {
                                    continue;
                                }
                                if let Some(slug) = item.get("slug").and_then(|v| v.as_str()) {
                                    let label = item
                                        .get("display_name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(slug);
                                    out.push(ModelOption {
                                        id: slug.to_string(),
                                        label: label.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            if out.is_empty() {
                for (id, label) in [
                    ("o3", "o3"),
                    ("o4-mini", "o4-mini"),
                    ("gpt-5", "GPT-5"),
                    ("gpt-5-codex", "GPT-5 Codex"),
                ] {
                    out.push(ModelOption {
                        id: id.to_string(),
                        label: label.to_string(),
                    });
                }
            }
            Ok(out)
        }
        "claude" => {
            let mut out: Vec<ModelOption> = vec![
                ModelOption {
                    id: "opus".to_string(),
                    label: "Claude Opus (Latest)".to_string(),
                },
                ModelOption {
                    id: "sonnet".to_string(),
                    label: "Claude Sonnet (Latest)".to_string(),
                },
                ModelOption {
                    id: "haiku".to_string(),
                    label: "Claude Haiku (Latest)".to_string(),
                },
                ModelOption {
                    id: "fable".to_string(),
                    label: "Claude Fable (Latest)".to_string(),
                },
            ];
            if let Some(home) = user_home_dir() {
                let settings_path = home.join(".claude").join("settings.json");
                if let Ok(raw) = tokio::fs::read_to_string(&settings_path).await {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                        if let Some(arr) = val.get("availableModels").and_then(|v| v.as_array()) {
                            for item in arr {
                                if let Some(m) = item.as_str() {
                                    if !m.trim().is_empty() && !out.iter().any(|x| x.id == m) {
                                        out.push(ModelOption {
                                            id: m.to_string(),
                                            label: m.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                        if let Some(cur) = val.get("model").and_then(|v| v.as_str()) {
                            let c = cur.trim();
                            if !c.is_empty() && !out.iter().any(|x| x.id == c) {
                                out.insert(
                                    0,
                                    ModelOption {
                                        id: c.to_string(),
                                        label: format!("{c} (当前配置)"),
                                    },
                                );
                            }
                        }
                    }
                }
            }
            Ok(out)
        }
        _ => Ok(Vec::new()),
    }
}

const WV2_LEAN_ARGS: &str = "--disable-gpu --js-flags=--optimize-for-size --disable-features=BackForwardCache,Translate,MediaRouter,OptimizationHints,msWebOOUI,msPdfOOUI --disable-background-networking --disable-component-update --disable-extensions --disable-sync --disable-default-apps --no-pings --disable-speech-api --renderer-process-limit=1";

#[cfg(target_os = "windows")]
fn set_webview_memory_level(win: &tauri::WebviewWindow, low: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
    };
    use windows_core::Interface;

    let _ = win.with_webview(move |wv| unsafe {
        if let Ok(core) = wv.controller().CoreWebView2() {
            if let Ok(core19) = core.cast::<ICoreWebView2_19>() {
                let level = if low {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                } else {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                };
                let _ = core19.SetMemoryUsageTargetLevel(level);
            }
        }
    });
}

fn attach_background_memory_trim(win: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        use std::sync::atomic::AtomicU64;
        use std::sync::OnceLock;

        static GLOBAL_FOCUS_GEN: OnceLock<AtomicU64> = OnceLock::new();
        let gen = GLOBAL_FOCUS_GEN.get_or_init(|| AtomicU64::new(0));
        let delay_secs: u64 = std::env::var("MARK5_LOW_MEM_DELAY_SECS")
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(20);
        let app_handle = win.app_handle().clone();
        win.on_window_event(move |ev| {
            let should_check = match ev {
                tauri::WindowEvent::Focused(true) => {
                    gen.fetch_add(1, Ordering::SeqCst);
                    for w in app_handle.webview_windows().values() {
                        set_webview_memory_level(w, false);
                    }
                    false
                }
                tauri::WindowEvent::Focused(false) | tauri::WindowEvent::Resized(_) => true,
                _ => false,
            };
            if should_check {
                let any_active = app_handle
                    .webview_windows()
                    .values()
                    .any(|w| w.is_focused().unwrap_or(false) && !w.is_minimized().unwrap_or(false));
                if any_active {
                    return;
                }
                let seq = gen.fetch_add(1, Ordering::SeqCst) + 1;
                let app_c = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                    let current_gen = GLOBAL_FOCUS_GEN
                        .get()
                        .map(|g| g.load(Ordering::SeqCst))
                        .unwrap_or(0);
                    if current_gen != seq {
                        return;
                    }
                    let windows = app_c.webview_windows();
                    let still_active = windows
                        .values()
                        .any(|w| w.is_focused().unwrap_or(false) && !w.is_minimized().unwrap_or(false));
                    if !still_active {
                        for w in windows.values() {
                            set_webview_memory_level(w, true);
                        }
                    }
                });
            }
        });
    }
    #[cfg(not(target_os = "windows"))]
    let _ = win;
}

fn schedule_window_show_fallback(win: &tauri::WebviewWindow) {
    let win_c = win.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
        if !win_c.is_visible().unwrap_or(true) {
            let _ = win_c.show();
            let _ = win_c.set_focus();
        }
    });
}

#[tauri::command]
async fn show_current_window(window: tauri::WebviewWindow) -> Result<(), String> {
    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}

#[tauri::command]
async fn open_new_window(
    app: AppHandle,
    state: State<'_, AppState>,
    theme: Option<String>,
) -> Result<(), String> {
    let idx = state.win_counter.fetch_add(1, Ordering::SeqCst) + 1;
    let label = format!("mark5-win-{idx}");
    let effective_theme = theme
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(read_saved_theme_sync);
    let bg = theme_bg_color(&effective_theme);
    let mut builder = WebviewWindowBuilder::new(&app, label, WebviewUrl::App("index.html".into()))
        .title("Mark 5")
        .inner_size(1440.0, 900.0)
        .min_inner_size(1024.0, 680.0)
        .visible(false)
        .background_color(bg)
        .additional_browser_args(WV2_LEAN_ARGS);
    if let Some(native_theme) = theme_native_opt(&effective_theme) {
        builder = builder.theme(Some(native_theme));
    }
    let win = builder
        .build()
        .map_err(|e| format!("open_new_window failed: {e}"))?;
    attach_background_memory_trim(&win);
    schedule_window_show_fallback(&win);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let workspace = WorkspaceManager::new();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let handle = app.handle().clone();
            let on_doc_op = Arc::new(move |op: serde_json::Value| {
                let _ = handle.emit("mc:agent-doc-op", op);
            });
            let mcp = DocMcpServer::new(Arc::clone(&workspace), on_doc_op);
            let agy = AgyAdapter::new(Arc::clone(&mcp), Arc::clone(&workspace));

            let mcp_start = Arc::clone(&mcp);
            tauri::async_runtime::spawn(async move {
                let _ = mcp_start.start().await;
            });

            app.manage(AppState {
                workspace: Arc::clone(&workspace),
                mcp,
                agy,
                win_counter: AtomicUsize::new(1),
            });

            if let Some(main_win) = app.get_webview_window("main") {
                let saved_theme = read_saved_theme_sync();
                let _ = main_win.set_background_color(Some(theme_bg_color(&saved_theme)));
                let _ = main_win.set_theme(theme_native_opt(&saved_theme));
                attach_background_memory_trim(&main_win);
                schedule_window_show_fallback(&main_win);
            }

            // Optional profiling harness hook: spawn N in-app windows deterministically
            if let Ok(n_str) = std::env::var("MARK5_BENCH_WINDOWS") {
                if let Ok(n) = n_str.trim().parse::<usize>() {
                    for idx in 2..=n.min(6) {
                        let label = format!("mark5-win-{idx}");
                        if let Ok(w) = WebviewWindowBuilder::new(
                            app,
                            label,
                            WebviewUrl::App("index.html".into()),
                        )
                        .title("Mark 5")
                        .inner_size(1440.0, 900.0)
                        .min_inner_size(1024.0, 680.0)
                        .additional_browser_args(WV2_LEAN_ARGS)
                        .build()
                        {
                            attach_background_memory_trim(&w);
                        }
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_default_workspace,
            pick_workspace_folder,
            list_workspace_files,
            list_workspace_all_files,
            read_markdown_file,
            write_markdown_file,
            rename_entry,
            duplicate_file,
            delete_entry,
            create_folder,
            reveal_in_explorer,
            sync_live_document,
            list_sessions,
            create_session,
            save_session,
            delete_session,
            list_threads,
            save_threads,
            list_history,
            save_history_snapshot,
            send_agent_turn,
            abort_agent_turn,
            export_document,
            check_cli_status,
            list_engine_models,
            open_new_window,
            set_app_theme,
            show_current_window,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Mark 5 Tauri app");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = app_handle.try_state::<AppState>() {
                let agy = Arc::clone(&state.agy);
                tauri::async_runtime::block_on(async move {
                    agy.abort_all().await;
                });
                state.mcp.stop_sync();
            }
        }
    });
}
