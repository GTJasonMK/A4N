use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use tera::Tera;
use tokio::sync::Mutex;

use a4n_core::error::AppError;

use crate::engine::{PromptEntry, PromptVersion};

#[derive(Debug, Clone)]
pub(super) struct ProjectOverride {
    pub(super) content: String,
}

/// 从 Markdown 文件加载默认提示词，支持模板渲染、项目覆盖和版本管理。
pub struct TeraPromptEngine {
    pub(super) entries: Arc<RwLock<HashMap<String, PromptEntry>>>,
    pub(super) original_contents: Arc<RwLock<HashMap<String, String>>>,
    pub(super) overrides: Arc<RwLock<HashMap<(i64, String), ProjectOverride>>>,
    pub(super) version_log: Arc<RwLock<HashMap<String, Vec<PromptVersion>>>>,
    pub(super) update_lock: Arc<Mutex<()>>,
    pub(super) git_store: Option<Arc<dyn crate::version_store::PromptVersionStore>>,
    pub(super) tera: Arc<RwLock<Tera>>,
    pub(super) defaults_dir: Arc<RwLock<Option<PathBuf>>>,
    pub(super) registry: Arc<RwLock<crate::registry::PromptRegistry>>,
}

impl TeraPromptEngine {
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
            registry: Arc::new(RwLock::new(crate::registry::PromptRegistry::empty())),
        }
    }

    #[cfg(any(test, feature = "test-support"))]
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
                    .map_err(|error| {
                        AppError::Internal(anyhow::anyhow!("模板编译失败 {}: {error}", entry.name))
                    })?;
                originals.insert(entry.name.clone(), entry.content.clone());
                entries.insert(entry.name.clone(), entry);
            }
        }
        Ok(engine)
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
