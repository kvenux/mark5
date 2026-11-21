use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::types::{CommentThread, FileEntry, HistorySnapshot, SessionRecord};

const META_DIR: &str = ".mark5";

pub struct WorkspaceManager {
    live_docs: RwLock<HashMap<String, String>>,
    write_lock: Mutex<()>,
}

impl WorkspaceManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            live_docs: RwLock::new(HashMap::new()),
            write_lock: Mutex::new(()),
        })
    }

    fn key_of(root_path: &str, rel_path: &str) -> String {
        let p = Path::new(root_path).join(rel_path);
        p.to_string_lossy().replace('\\', "/").to_lowercase()
    }

    pub async fn set_live_markdown(&self, root_path: &str, rel_path: &str, content: String) {
        let mut map = self.live_docs.write().await;
        map.insert(Self::key_of(root_path, rel_path), content);
    }

    pub async fn get_latest_markdown(&self, root_path: &str, rel_path: &str) -> String {
        {
            let map = self.live_docs.read().await;
            if let Some(cached) = map.get(&Self::key_of(root_path, rel_path)) {
                return cached.clone();
            }
        }
        self.read_markdown_file(root_path, rel_path).await
    }

    async fn atomic_write(&self, full_path: &Path, content: &str) -> Result<(), String> {
        let _guard = self.write_lock.lock().await;
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("create_dir_all failed: {e}"))?;
        }
        let tmp_name = format!(
            "{}.{}.tmp",
            full_path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
            Uuid::new_v4().simple()
        );
        let tmp_path = full_path.with_file_name(tmp_name);
        fs::write(&tmp_path, content.as_bytes())
            .await
            .map_err(|e| format!("write tmp failed: {e}"))?;
        if let Err(e) = fs::rename(&tmp_path, full_path).await {
            // Fallback on Windows if target is momentarily locked
            let _ = fs::remove_file(&tmp_path).await;
            fs::write(full_path, content.as_bytes())
                .await
                .map_err(|e2| format!("write fallback failed ({e}): {e2}"))?;
        }
        Ok(())
    }

    async fn ensure_meta_dir(&self, root_path: &str) -> Result<PathBuf, String> {
        let dir = Path::new(root_path).join(META_DIR);
        fs::create_dir_all(&dir)
            .await
            .map_err(|e| format!("ensure meta dir failed: {e}"))?;
        Ok(dir)
    }

    pub async fn list_markdown_files(&self, root_path: &str) -> Vec<FileEntry> {
        self.list_md_recursive(Path::new(root_path), "", 0).await
    }

    fn list_md_recursive<'a>(
        &'a self,
        root: &'a Path,
        current_rel: &'a str,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<FileEntry>> + Send + 'a>> {
        Box::pin(async move {
            if depth > 8 {
                return vec![];
            }
            let abs_dir = if current_rel.is_empty() {
                root.to_path_buf()
            } else {
                root.join(current_rel)
            };
            let mut entries = match fs::read_dir(&abs_dir).await {
                Ok(e) => e,
                Err(_) => return vec![],
            };
            let mut result = Vec::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name == "node_modules" || name == "out" || name == "target" {
                    continue;
                }
                let rel_path = if current_rel.is_empty() {
                    name.clone()
                } else {
                    format!("{current_rel}/{name}")
                };
                if let Ok(ft) = entry.file_type().await {
                    if ft.is_dir() {
                        let children = self.list_md_recursive(root, &rel_path, depth + 1).await;
                        if !children.is_empty() {
                            result.push(FileEntry {
                                name,
                                rel_path,
                                is_dir: true,
                                children: Some(children),
                            });
                        }
                    } else {
                        let lower = name.to_lowercase();
                        if lower.ends_with(".md") || lower.ends_with(".markdown") {
                            result.push(FileEntry {
                                name,
                                rel_path,
                                is_dir: false,
                                children: None,
                            });
                        }
                    }
                }
            }
            result.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });
            result
        })
    }

    pub async fn list_all_files(&self, root_path: &str) -> Vec<FileEntry> {
        let ignored: HashSet<&'static str> = [
            ".git",
            ".mark5",
            ".mark-carbon",
            "node_modules",
            "out",
            "dist",
            "target",
            ".next",
            ".cache",
            "build",
            "coverage",
            "__pycache__",
        ]
        .into_iter()
        .collect();
        self.list_all_recursive(Path::new(root_path), "", 0, &ignored)
            .await
    }

    fn list_all_recursive<'a>(
        &'a self,
        root: &'a Path,
        current_rel: &'a str,
        depth: usize,
        ignored: &'a HashSet<&'static str>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<FileEntry>> + Send + 'a>> {
        Box::pin(async move {
            if depth > 6 {
                return vec![];
            }
            let abs_dir = if current_rel.is_empty() {
                root.to_path_buf()
            } else {
                root.join(current_rel)
            };
            let mut entries = match fs::read_dir(&abs_dir).await {
                Ok(e) => e,
                Err(_) => return vec![],
            };
            let mut result = Vec::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let rel_path = if current_rel.is_empty() {
                    name.clone()
                } else {
                    format!("{current_rel}/{name}")
                };
                if let Ok(ft) = entry.file_type().await {
                    if ft.is_dir() {
                        if ignored.contains(name.as_str()) {
                            continue;
                        }
                        let children = self
                            .list_all_recursive(root, &rel_path, depth + 1, ignored)
                            .await;
                        result.push(FileEntry {
                            name,
                            rel_path,
                            is_dir: true,
                            children: Some(children),
                        });
                    } else {
                        result.push(FileEntry {
                            name,
                            rel_path,
                            is_dir: false,
                            children: None,
                        });
                    }
                }
            }
            result.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });
            result
        })
    }

    pub async fn read_markdown_file(&self, root_path: &str, rel_path: &str) -> String {
        let full_path = Path::new(root_path).join(rel_path);
        match fs::read_to_string(&full_path).await {
            Ok(content) => {
                self.set_live_markdown(root_path, rel_path, content.clone())
                    .await;
                content
            }
            Err(_) => String::new(),
        }
    }

    pub async fn write_markdown_file(
        &self,
        root_path: &str,
        rel_path: &str,
        content: &str,
    ) -> Result<(), String> {
        let full_path = Path::new(root_path).join(rel_path);
        self.set_live_markdown(root_path, rel_path, content.to_string())
            .await;
        self.atomic_write(&full_path, content).await
    }

    fn validate_segment(name: &str) -> Result<String, String> {
        let trimmed = name.trim();
        if trimmed.is_empty()
            || trimmed == "."
            || trimmed == ".."
            || trimmed.contains('/')
            || trimmed.contains('\\')
            || trimmed.contains(':')
            || trimmed.contains('*')
            || trimmed.contains('?')
            || trimmed.contains('"')
            || trimmed.contains('<')
            || trimmed.contains('>')
            || trimmed.contains('|')
        {
            return Err("名称包含非法字符或为空".to_string());
        }
        Ok(trimmed.to_string())
    }

    pub async fn rename_entry(
        &self,
        root_path: &str,
        old_rel_path: &str,
        new_name: &str,
    ) -> Result<String, String> {
        let clean_name = Self::validate_segment(new_name)?;
        let old_full = Path::new(root_path).join(old_rel_path);
        if !old_full.exists() {
            return Err("源文件或目录不存在".to_string());
        }
        let parent_rel = Path::new(old_rel_path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let new_rel = if parent_rel.is_empty() {
            clean_name.clone()
        } else {
            format!("{parent_rel}/{clean_name}")
        };
        if new_rel == old_rel_path {
            return Ok(new_rel);
        }
        let new_full = Path::new(root_path).join(&new_rel);
        if new_full.exists() {
            return Err(format!("目标已存在：{clean_name}"));
        }
        fs::rename(&old_full, &new_full)
            .await
            .map_err(|e| format!("重命名失败：{e}"))?;

        let old_key = Self::key_of(root_path, old_rel_path);
        let new_key = Self::key_of(root_path, &new_rel);
        let mut live = self.live_docs.write().await;
        if let Some(content) = live.remove(&old_key) {
            live.insert(new_key, content);
        }
        Ok(new_rel)
    }

    pub async fn duplicate_file(
        &self,
        root_path: &str,
        rel_path: &str,
    ) -> Result<String, String> {
        let src_full = Path::new(root_path).join(rel_path);
        if !src_full.is_file() {
            return Err("仅支持复制文件".to_string());
        }
        let parent_rel = Path::new(rel_path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let stem = src_full
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let ext = src_full
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();

        let mut idx = 1usize;
        let new_rel = loop {
            let suffix = if idx == 1 {
                "-副本".to_string()
            } else {
                format!("-副本{idx}")
            };
            let cand_name = format!("{stem}{suffix}{ext}");
            let cand_rel = if parent_rel.is_empty() {
                cand_name
            } else {
                format!("{parent_rel}/{cand_name}")
            };
            if !Path::new(root_path).join(&cand_rel).exists() {
                break cand_rel;
            }
            idx += 1;
        };

        let dst_full = Path::new(root_path).join(&new_rel);
        fs::copy(&src_full, &dst_full)
            .await
            .map_err(|e| format!("复制文件失败：{e}"))?;
        Ok(new_rel)
    }

    pub async fn delete_entry(&self, root_path: &str, rel_path: &str) -> Result<(), String> {
        if rel_path.trim().is_empty() || rel_path.contains("..") {
            return Err("非法路径".to_string());
        }
        let full = Path::new(root_path).join(rel_path);
        if !full.exists() {
            return Ok(());
        }
        if full.is_dir() {
            fs::remove_dir_all(&full)
                .await
                .map_err(|e| format!("删除目录失败：{e}"))?;
        } else {
            fs::remove_file(&full)
                .await
                .map_err(|e| format!("删除文件失败：{e}"))?;
        }
        let key = Self::key_of(root_path, rel_path);
        self.live_docs.write().await.remove(&key);
        Ok(())
    }

    pub async fn create_folder(
        &self,
        root_path: &str,
        parent_rel_path: &str,
        folder_name: &str,
    ) -> Result<String, String> {
        let clean_name = Self::validate_segment(folder_name)?;
        let new_rel = if parent_rel_path.trim().is_empty() {
            clean_name
        } else {
            format!("{}/{}", parent_rel_path.trim_end_matches('/'), clean_name)
        };
        let full = Path::new(root_path).join(&new_rel);
        fs::create_dir_all(&full)
            .await
            .map_err(|e| format!("创建目录失败：{e}"))?;
        Ok(new_rel)
    }

    // ---------- Sessions ----------
    pub async fn list_sessions(&self, root_path: &str) -> Vec<SessionRecord> {
        let meta = match self.ensure_meta_dir(root_path).await {
            Ok(m) => m,
            Err(_) => return vec![],
        };
        let file = meta.join("sessions.json");
        let raw = match fs::read_to_string(&file).await {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        let mut list: Vec<SessionRecord> = serde_json::from_str(&raw).unwrap_or_default();
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        list
    }

    pub async fn save_session(&self, root_path: &str, mut session: SessionRecord) -> Result<(), String> {
        let meta = self.ensure_meta_dir(root_path).await?;
        let file = meta.join("sessions.json");
        let mut list = self.list_sessions(root_path).await;
        if let Some(pos) = list.iter().position(|s| s.id == session.id) {
            if session.conversation_id.is_none() {
                session.conversation_id = list[pos].conversation_id.clone();
            }
            list[pos] = session;
        } else {
            list.insert(0, session);
        }
        let body = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
        self.atomic_write(&file, &body).await
    }

    pub async fn update_session_conversation_id(
        &self,
        root_path: &str,
        session_id: &str,
        conversation_id: &str,
    ) -> Result<(), String> {
        let meta = self.ensure_meta_dir(root_path).await?;
        let file = meta.join("sessions.json");
        let mut list = self.list_sessions(root_path).await;
        if let Some(s) = list.iter_mut().find(|s| s.id == session_id) {
            s.conversation_id = Some(conversation_id.to_string());
            let body = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
            self.atomic_write(&file, &body).await?;
        }
        Ok(())
    }

    pub async fn delete_session(&self, root_path: &str, session_id: &str) -> Result<(), String> {
        let meta = self.ensure_meta_dir(root_path).await?;
        let file = meta.join("sessions.json");
        let list: Vec<SessionRecord> = self
            .list_sessions(root_path)
            .await
            .into_iter()
            .filter(|s| s.id != session_id)
            .collect();
        let body = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
        self.atomic_write(&file, &body).await
    }

    pub async fn get_session(&self, root_path: &str, session_id: &str) -> Option<SessionRecord> {
        self.list_sessions(root_path)
            .await
            .into_iter()
            .find(|s| s.id == session_id)
    }

    // ---------- Threads ----------
    fn safe_doc_key(doc_rel_path: &str) -> String {
        doc_rel_path
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c > '\x7f' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    pub async fn list_threads(&self, root_path: &str, doc_rel_path: &str) -> Vec<CommentThread> {
        let meta = match self.ensure_meta_dir(root_path).await {
            Ok(m) => m,
            Err(_) => return vec![],
        };
        let file = meta
            .join("threads")
            .join(format!("{}.json", Self::safe_doc_key(doc_rel_path)));
        let raw = match fs::read_to_string(&file).await {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub async fn save_threads(
        &self,
        root_path: &str,
        doc_rel_path: &str,
        threads: Vec<CommentThread>,
    ) -> Result<(), String> {
        let meta = self.ensure_meta_dir(root_path).await?;
        let file = meta
            .join("threads")
            .join(format!("{}.json", Self::safe_doc_key(doc_rel_path)));
        let body = serde_json::to_string_pretty(&threads).map_err(|e| e.to_string())?;
        self.atomic_write(&file, &body).await
    }

    // ---------- History ----------
    pub async fn list_history(&self, root_path: &str, doc_rel_path: &str) -> Vec<HistorySnapshot> {
        let meta = match self.ensure_meta_dir(root_path).await {
            Ok(m) => m,
            Err(_) => return vec![],
        };
        let file = meta
            .join("history")
            .join(format!("{}.json", Self::safe_doc_key(doc_rel_path)));
        let raw = match fs::read_to_string(&file).await {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub async fn append_history(
        &self,
        root_path: &str,
        doc_rel_path: &str,
        actor: &str,
        summary: &str,
        markdown: &str,
    ) -> Result<HistorySnapshot, String> {
        let meta = self.ensure_meta_dir(root_path).await?;
        let file = meta
            .join("history")
            .join(format!("{}.json", Self::safe_doc_key(doc_rel_path)));
        let mut list = self.list_history(root_path, doc_rel_path).await;
        let snap = HistorySnapshot {
            id: format!("snap-{}", &Uuid::new_v4().simple().to_string()[..10]),
            doc_rel_path: doc_rel_path.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            actor: actor.to_string(),
            summary: summary.to_string(),
            markdown: markdown.to_string(),
        };
        list.insert(0, snap.clone());
        list.truncate(40);
        let body = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
        self.atomic_write(&file, &body).await?;
        Ok(snap)
    }
}

pub fn split_markdown_blocks(markdown: &str) -> Vec<String> {
    let normalized = markdown.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();
    let mut blocks: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut in_fence = false;

    for line in lines {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            current.push(line);
            continue;
        }
        if !in_fence && line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(current.join("\n"));
                current.clear();
            }
            continue;
        }
        current.push(line);
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}
