//! Tera 提示词引擎：基于 Tera 模板引擎的 PromptEngine 实现
//!
//! 从 Markdown 文件目录加载默认提示词，解析 frontmatter 中的模板本地字段，
//! 并从 `_registry.yaml` 注入统一元数据，使用 Tera 模板引擎渲染，支持项目级覆盖。

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tera::{Context, Tera};
use tokio::sync::Mutex;

use super::engine::{
    DependencyGraph, GraphEdge, GraphNode, PromptEngine, PromptEntry, PromptVersion,
};
use a4n_core::error::AppError;

mod git_persistence;
mod loader;
mod mutation;
mod variables;
mod version;

#[cfg(test)]
mod tests;

use variables::missing_template_variables;

/// 项目级提示词覆盖
#[derive(Debug, Clone)]
struct ProjectOverride {
    content: String,
}

/// Tera 提示词引擎
///
/// 从 Markdown 文件加载默认提示词，支持模板变量渲染、项目覆盖、版本管理。
pub struct TeraPromptEngine {
    /// 提示词条目（name → entry）
    entries: Arc<RwLock<HashMap<String, PromptEntry>>>,
    /// 原始模板内容（用于 reset）
    original_contents: Arc<RwLock<HashMap<String, String>>>,
    /// 项目级覆盖（project_id, name) → content
    overrides: Arc<RwLock<HashMap<(i64, String), ProjectOverride>>>,
    /// 版本历史（name → version log）
    version_log: Arc<RwLock<HashMap<String, Vec<PromptVersion>>>>,
    /// 串行化提示词变更的 Git 持久化和内存状态提交，避免提交前置后产生并发版本回退。
    update_lock: Arc<Mutex<()>>,
    /// 可选 Git 持久化层：开启后每次 update 自动 commit
    git_store: Option<Arc<crate::prompt::git_store::GitPromptStore>>,
    /// Tera 模板引擎实例
    tera: Arc<RwLock<Tera>>,
    /// 默认提示词目录（from_directory 时记录），供 load_defaults 增量重扫
    defaults_dir: Arc<RwLock<Option<PathBuf>>>,
    /// `_registry.yaml` 中央元数据（路径/used_by/dependencies）
    registry: Arc<RwLock<crate::prompt::registry::PromptRegistry>>,
}

impl TeraPromptEngine {
    /// 创建空的引擎
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            original_contents: Arc::new(RwLock::new(HashMap::new())),
            overrides: Arc::new(RwLock::new(HashMap::new())),
            version_log: Arc::new(RwLock::new(HashMap::new())),
            update_lock: Arc::new(Mutex::new(())),
            git_store: None,
            tera: Arc::new(RwLock::new(Tera::default())),
            defaults_dir: Arc::new(RwLock::new(None)),
            registry: Arc::new(RwLock::new(crate::prompt::registry::PromptRegistry::empty())),
        }
    }

    /// 从内置提示词集合初始化
    pub fn with_defaults(defaults: Vec<PromptEntry>) -> Result<Self, AppError> {
        let engine = Self::new();
        {
            let mut entries = engine.entries.write();
            let mut originals = engine.original_contents.write();
            for entry in defaults {
                engine
                    .tera
                    .write()
                    .add_raw_template(&entry.name, &entry.content)
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("模板编译失败 {}: {e}", entry.name))
                    })?;
                originals.insert(entry.name.clone(), entry.content.clone());
                entries.insert(entry.name.clone(), entry);
            }
        }
        Ok(engine)
    }

    /// 获取实际内容（项目覆盖优先）
    fn resolve_content(
        &self,
        name: &str,
        project_id: Option<i64>,
    ) -> Result<(String, bool), AppError> {
        let entry_content = {
            let entries = self.entries.read();
            let entry = entries
                .get(name)
                .ok_or_else(|| Self::prompt_not_found(name))?;
            entry.content.clone()
        };

        if let Some(pid) = project_id {
            Self::validate_override_project_id(pid)?;
            let overrides = self.overrides.read();
            if let Some(ov) = overrides.get(&(pid, name.to_string())) {
                return Ok((ov.content.clone(), true));
            }
        }

        Ok((entry_content, false))
    }

    fn normalize_override_identity(project_id: i64, name: &str) -> Result<(i64, String), AppError> {
        let project_id = Self::validate_override_project_id(project_id)?;

        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::InvalidArgument(
                "提示词覆盖 name 不能为空".to_string(),
            ));
        }
        if name.chars().any(char::is_control) {
            return Err(AppError::InvalidArgument(
                "提示词覆盖 name 不能包含控制字符".to_string(),
            ));
        }

        Ok((project_id, name.to_string()))
    }

    fn validate_override_project_id(project_id: i64) -> Result<i64, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        Ok(project_id)
    }

    fn validate_override_content(content: &str) -> Result<(), AppError> {
        if content.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "提示词覆盖内容不能为空".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_override_template(name: &str, content: &str) -> Result<(), AppError> {
        let mut tera = Tera::default();
        tera.add_raw_template(name, content)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败: {e}")))?;
        Ok(())
    }

    fn prepare_project_override(
        &self,
        project_id: i64,
        name: &str,
        content: String,
    ) -> Result<(i64, String, String), AppError> {
        let (project_id, name) = Self::normalize_override_identity(project_id, name)?;
        Self::validate_override_content(&content)?;
        if !self.entries.read().contains_key(&name) {
            return Err(Self::prompt_not_found(&name));
        }
        Self::validate_override_template(&name, &content)?;

        Ok((project_id, name, content))
    }

    pub fn hydrate_project_overrides(
        &self,
        overrides: Vec<(i64, String, String)>,
    ) -> Result<usize, AppError> {
        let loaded = overrides.len();
        let mut prepared = Vec::with_capacity(overrides.len());
        for (project_id, name, content) in overrides {
            prepared.push(self.prepare_project_override(project_id, &name, content)?);
        }

        let mut runtime_overrides = self.overrides.write();
        for (project_id, name, content) in prepared {
            runtime_overrides.insert((project_id, name), ProjectOverride { content });
        }

        Ok(loaded)
    }

    pub(super) fn prompt_not_found(name: &str) -> AppError {
        AppError::NotFound(format!("提示词不存在：{name}"))
    }
}

impl Default for TeraPromptEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PromptEngine for TeraPromptEngine {
    fn render(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let (content, is_override) = self.resolve_content(template_name, project_id)?;

        let mut context = Context::new();
        for (key, val) in variables {
            context.insert(key, val);
        }

        if is_override {
            let mut tera = Tera::default();
            tera.add_raw_template(template_name, &content)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败: {e}")))?;
            tera.render(template_name, &context)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("模板渲染失败: {e}")))
        } else {
            self.tera
                .read()
                .render(template_name, &context)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("模板渲染失败: {e}")))
        }
    }

    fn get(&self, name: &str) -> Result<PromptEntry, AppError> {
        self.entries
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| Self::prompt_not_found(name))
    }

    fn list_all(&self) -> Vec<PromptEntry> {
        self.entries.read().values().cloned().collect()
    }

    async fn load_defaults(&self) -> Result<usize, AppError> {
        self.reload_defaults_from_directory().await
    }

    async fn update(
        &self,
        name: &str,
        content: String,
        author: &str,
    ) -> Result<PromptEntry, AppError> {
        self.update_prompt(name, content, author).await
    }

    async fn update_many(
        &self,
        updates: Vec<(String, String)>,
        author: &str,
    ) -> Result<Vec<PromptEntry>, AppError> {
        self.update_prompts(updates, author).await
    }

    async fn reset(&self, name: &str) -> Result<(), AppError> {
        self.reset_prompt(name).await
    }

    fn version_history(&self, name: &str) -> Result<Vec<PromptVersion>, AppError> {
        let entries = self.entries.read();
        let entry = entries
            .get(name)
            .ok_or_else(|| Self::prompt_not_found(name))?;

        let mut history = self
            .version_log
            .read()
            .get(name)
            .cloned()
            .unwrap_or_default();

        // 追加当前版本
        history.push(PromptVersion {
            version: entry.version.clone(),
            timestamp: entry.modified_at,
            author: "current".into(),
            message: if entry.is_modified {
                "当前修改版本".into()
            } else {
                "原始版本".into()
            },
            content_snapshot: Some(entry.content.clone()),
        });

        Ok(history)
    }

    fn get_version(&self, name: &str, version: &str) -> Result<PromptEntry, AppError> {
        // 先检查是否匹配当前版本
        let entries = self.entries.read();
        let current = entries
            .get(name)
            .ok_or_else(|| Self::prompt_not_found(name))?;

        if current.version == version {
            return Ok(current.clone());
        }

        // 在版本历史中查找
        let log = self.version_log.read();
        if let Some(history) = log.get(name) {
            for v in history {
                if v.version == version
                    && let Some(ref content) = v.content_snapshot
                {
                    let mut entry = current.clone();
                    entry.content = content.clone();
                    entry.version = v.version.clone();
                    entry.modified_at = v.timestamp;
                    return Ok(entry);
                }
            }
        }

        Err(AppError::NotFound(format!(
            "版本 {version} 不存在于提示词 {name}"
        )))
    }

    async fn set_project_override(
        &self,
        project_id: i64,
        name: &str,
        content: String,
    ) -> Result<(), AppError> {
        let (project_id, name, content) =
            self.prepare_project_override(project_id, name, content)?;

        self.overrides
            .write()
            .insert((project_id, name), ProjectOverride { content });
        Ok(())
    }

    async fn remove_project_override(&self, project_id: i64, name: &str) -> Result<(), AppError> {
        let (project_id, name) = Self::normalize_override_identity(project_id, name)?;
        self.overrides.write().remove(&(project_id, name));
        Ok(())
    }

    fn diff(&self, name: &str, version_a: &str, version_b: &str) -> Result<String, AppError> {
        // 获取两个版本的完整内容
        let entries = self.entries.read();
        let current = entries
            .get(name)
            .ok_or_else(|| Self::prompt_not_found(name))?;

        let content_a = if current.version == version_a {
            current.content.clone()
        } else {
            self.version_log
                .read()
                .get(name)
                .and_then(|h| h.iter().find(|v| v.version == version_a))
                .and_then(|v| v.content_snapshot.clone())
                .ok_or_else(|| AppError::NotFound(format!("版本 {version_a} 不存在")))?
        };

        let content_b = if current.version == version_b {
            current.content.clone()
        } else {
            self.version_log
                .read()
                .get(name)
                .and_then(|h| h.iter().find(|v| v.version == version_b))
                .and_then(|v| v.content_snapshot.clone())
                .ok_or_else(|| AppError::NotFound(format!("版本 {version_b} 不存在")))?
        };

        // 使用 similar 生成差异
        let diff = similar::TextDiff::from_lines(&content_a, &content_b);
        let result = diff
            .unified_diff()
            .header(
                &format!("{}[{}]", name, version_a),
                &format!("{}[{}]", name, version_b),
            )
            .to_string();

        Ok(result)
    }

    fn validate_variables(
        &self,
        template_name: &str,
        variables: &HashSet<String>,
    ) -> Result<Vec<String>, AppError> {
        let entries = self.entries.read();
        let entry = entries
            .get(template_name)
            .ok_or_else(|| Self::prompt_not_found(template_name))?;

        missing_template_variables(&entry.content, variables)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("模板变量解析失败: {e}")))
    }

    fn dependency_graph(&self) -> DependencyGraph {
        let entries = self.entries.read();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for entry in entries.values() {
            nodes.push(GraphNode {
                name: entry.name.clone(),
                category: entry.category.clone(),
            });
            for dep in &entry.dependencies {
                edges.push(GraphEdge {
                    from: entry.name.clone(),
                    to: dep.clone(),
                });
            }
        }

        DependencyGraph { nodes, edges }
    }

    fn render_ab(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<(String, String), AppError> {
        let current = self.render(template_name, variables, project_id)?;

        let original_content = self
            .original_contents
            .read()
            .get(template_name)
            .cloned()
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("提示词缺少原始内容：{template_name}"))
            })?;
        let mut ctx = Context::new();
        for (key, value) in variables {
            ctx.insert(key, value);
        }
        let mut original_tera = Tera::default();
        original_tera
            .add_raw_template(template_name, &original_content)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败: {e}")))?;
        let original = original_tera
            .render(template_name, &ctx)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("渲染失败: {e}")))?;

        Ok((current, original))
    }

    fn registry(&self) -> Option<Arc<RwLock<crate::prompt::registry::PromptRegistry>>> {
        Some(self.registry.clone())
    }
}
