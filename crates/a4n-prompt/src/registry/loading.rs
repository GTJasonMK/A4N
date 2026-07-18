use std::fs;
use std::path::Path;

use a4n_core::error::AppError;

use super::{PromptRegistry, RegistryFile};

impl PromptRegistry {
    /// 从 prompts 目录加载 `_registry.yaml`。
    pub fn load_from_dir(prompts_dir: &Path) -> Result<Self, AppError> {
        let path = prompts_dir.join("_registry.yaml");
        if !path.is_file() {
            return Err(AppError::ConfigError(format!(
                "缺少提示词注册表：{}",
                path.display()
            )));
        }

        let text = fs::read_to_string(&path)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("读取注册表 {path:?} 失败：{e}")))?;

        let file: RegistryFile = serde_yaml::from_str(&text)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("解析注册表 YAML 失败：{e}")))?;

        tracing::info!(
            "已加载提示词注册表：{} 个 prompts / {} 个 categories",
            file.prompts.len(),
            file.categories.len(),
        );

        Ok(Self { file, loaded: true })
    }
}
