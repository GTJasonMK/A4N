use a4n_core::error::AppError;

use crate::engine::PromptVersion;

use super::super::TeraPromptEngine;
use super::super::version::bump_version;

impl TeraPromptEngine {
    pub async fn reset(&self, name: &str) -> Result<(), AppError> {
        let _update_guard = self.update_lock.lock().await;

        let current = {
            let entries = self.entries.read();
            entries
                .get(name)
                .cloned()
                .ok_or_else(|| Self::prompt_not_found(name))?
        };
        let orig_content = {
            let originals = self.original_contents.read();
            originals.get(name).cloned()
        }
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("提示词缺少原始内容：{name}")))?;

        {
            let mut staged_tera = self.tera.read().clone();
            staged_tera
                .add_raw_template(name, &orig_content)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败: {e}")))?;
        }
        if current.content == orig_content && !current.is_modified {
            return Ok(());
        }

        let new_version = bump_version(&current.version);
        if let Some(store) = &self.git_store {
            let message = format!("system: 重置 {name} → v{new_version}");
            let store_clone = store.clone();
            let name_clone = name.to_string();
            let commit_content = orig_content.clone();
            tokio::task::spawn_blocking(move || {
                store_clone.commit_prompt(&name_clone, &commit_content, &message)
            })
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("git commit task 失败: {e}")))??;
        }

        {
            let mut entries = self.entries.write();
            let entry = entries
                .get_mut(name)
                .ok_or_else(|| Self::prompt_not_found(name))?;
            let prev_version = PromptVersion {
                version: entry.version.clone(),
                timestamp: entry.modified_at,
                author: "system".into(),
                message: format!("重置前版本: {}", entry.version),
                content_snapshot: Some(entry.content.clone()),
            };
            self.tera
                .write()
                .add_raw_template(name, &orig_content)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败: {e}")))?;
            self.version_log
                .write()
                .entry(name.to_string())
                .or_default()
                .push(prev_version);
            entry.version = new_version;
            entry.content = orig_content;
            entry.is_modified = false;
            entry.modified_at = chrono::Utc::now().naive_utc();
        }
        Ok(())
    }
}
