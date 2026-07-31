use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::{lock_config, write_private_json_unlocked};

const CONTEXT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SessionContext {
    pub(crate) schema_version: u32,
    pub(crate) server_fingerprint: Option<String>,
    pub(crate) user_id: Option<i64>,
    pub(crate) project_id: Option<i64>,
    pub(crate) chapter_id: Option<i64>,
    pub(crate) snapshot_id: Option<i64>,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Default for SessionContext {
    fn default() -> Self {
        Self {
            schema_version: CONTEXT_SCHEMA_VERSION,
            server_fingerprint: None,
            user_id: None,
            project_id: None,
            chapter_id: None,
            snapshot_id: None,
            path: None,
        }
    }
}

impl SessionContext {
    pub(crate) fn load(path: Option<PathBuf>, server: &str) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self {
                server_fingerprint: Some(server.to_string()),
                ..Self::default()
            });
        };
        let _lock = lock_config(&path)?;
        let mut context = Self::read_unlocked(&path)?;
        if let Some(bound) = context.server_fingerprint.as_deref()
            && bound != server
        {
            bail!("CLI 上下文属于其他 Server: expected={bound} actual={server}");
        }
        context.server_fingerprint = Some(server.to_string());
        context.path = Some(path);
        context.validate()?;
        Ok(context)
    }

    pub(crate) fn bind_user(&mut self, user_id: i64) -> Result<()> {
        validate_positive(user_id, "user_id")?;
        if let Some(bound) = self.user_id
            && bound != user_id
        {
            bail!("CLI 上下文属于其他用户: expected={bound} actual={user_id}");
        }
        if self.user_id.is_none() {
            let mut next = self.clone();
            next.user_id = Some(user_id);
            self.commit(next)?;
        }
        Ok(())
    }

    pub(crate) fn set(
        &mut self,
        project_id: Option<i64>,
        chapter_id: Option<i64>,
        snapshot_id: Option<i64>,
    ) -> Result<()> {
        for (value, field) in [
            (project_id, "project_id"),
            (chapter_id, "chapter_id"),
            (snapshot_id, "snapshot_id"),
        ] {
            if let Some(value) = value {
                validate_positive(value, field)?;
            }
        }
        let mut next = self.clone();
        if let Some(project_id) = project_id
            && next.project_id != Some(project_id)
        {
            next.project_id = Some(project_id);
            next.chapter_id = None;
            next.snapshot_id = None;
        }
        if let Some(chapter_id) = chapter_id
            && next.chapter_id != Some(chapter_id)
        {
            next.chapter_id = Some(chapter_id);
            next.snapshot_id = None;
        }
        if let Some(snapshot_id) = snapshot_id {
            next.snapshot_id = Some(snapshot_id);
        }
        self.commit(next)
    }

    pub(crate) fn clear(&mut self) -> Result<()> {
        self.commit(Self {
            server_fingerprint: self.server_fingerprint.clone(),
            path: self.path.clone(),
            ..Self::default()
        })
    }

    pub(crate) fn clear_project(&mut self) -> Result<()> {
        let mut next = self.clone();
        next.project_id = None;
        next.chapter_id = None;
        next.snapshot_id = None;
        self.commit(next)
    }

    pub(crate) fn clear_chapter(&mut self) -> Result<()> {
        let mut next = self.clone();
        next.chapter_id = None;
        next.snapshot_id = None;
        self.commit(next)
    }

    pub(crate) fn clear_snapshot(&mut self) -> Result<()> {
        let mut next = self.clone();
        next.snapshot_id = None;
        self.commit(next)
    }

    pub(crate) fn as_json(&self) -> Value {
        json!({
            "schema_version": self.schema_version,
            "server_fingerprint": self.server_fingerprint,
            "user_id": self.user_id,
            "project_id": self.project_id,
            "chapter_id": self.chapter_id,
            "snapshot_id": self.snapshot_id,
            "persistent": self.path.is_some(),
            "context_file": self.path,
        })
    }

    fn commit(&mut self, next: Self) -> Result<()> {
        next.validate()?;
        if let Some(path) = self.path.as_deref() {
            let _lock = lock_config(path)?;
            let mut current = Self::read_unlocked(path)?;
            if let Some(bound) = current.server_fingerprint.as_deref()
                && Some(bound) != self.server_fingerprint.as_deref()
            {
                bail!("CLI 上下文已被另一进程切换到其他 Server");
            }
            current.server_fingerprint = self.server_fingerprint.clone();
            current.path = self.path.clone();
            if current != *self {
                bail!("CLI 上下文已被另一进程修改；请重新运行当前命令");
            }
            write_private_json_unlocked(path, &next)?;
        }
        *self = next;
        Ok(())
    }

    fn read_unlocked(path: &Path) -> Result<Self> {
        let context = if path.exists() {
            let contents = fs::read_to_string(path)
                .with_context(|| format!("读取 CLI 上下文失败: {}", path.display()))?;
            serde_json::from_str::<Self>(&contents)
                .with_context(|| format!("CLI 上下文无效: {}", path.display()))?
        } else {
            Self::default()
        };
        if context.schema_version != CONTEXT_SCHEMA_VERSION {
            bail!(
                "CLI 上下文 schema 不兼容: expected={} actual={}",
                CONTEXT_SCHEMA_VERSION,
                context.schema_version
            );
        }
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> Result<()> {
        for (value, field) in [
            (self.user_id, "user_id"),
            (self.project_id, "project_id"),
            (self.chapter_id, "chapter_id"),
            (self.snapshot_id, "snapshot_id"),
        ] {
            if let Some(value) = value {
                validate_positive(value, field)?;
            }
        }
        if self.chapter_id.is_some() && self.project_id.is_none() {
            bail!("chapter_id 需要 project_id 上下文");
        }
        if self.snapshot_id.is_some() && self.project_id.is_none() {
            bail!("snapshot_id 需要 project_id 上下文");
        }
        Ok(())
    }
}

fn validate_positive(value: i64, field: &str) -> Result<()> {
    if value <= 0 {
        bail!("{field} 必须为正整数");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::SessionContext;

    #[test]
    fn context_clears_children_and_rejects_other_server() {
        let path = std::env::temp_dir().join(format!("a4n-context-{}.json", Uuid::new_v4()));
        let mut context = SessionContext::load(Some(path.clone()), "http://one/api").unwrap();
        context.set(Some(1), Some(2), Some(3)).unwrap();
        context.set(Some(4), None, None).unwrap();
        assert_eq!(context.project_id, Some(4));
        assert_eq!(context.chapter_id, None);
        assert_eq!(context.snapshot_id, None);
        assert!(SessionContext::load(Some(path.clone()), "http://two/api").is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn failed_persistence_does_not_mutate_in_memory_context() {
        let directory = TempDir::new().unwrap();
        let blocked_parent = directory.path().join("context");
        let path = blocked_parent.join("state.json");
        let mut context = SessionContext::load(Some(path.clone()), "http://one/api").unwrap();

        std::fs::remove_file(path.with_extension("json.lock")).unwrap();
        std::fs::remove_dir(&blocked_parent).unwrap();
        std::fs::write(&blocked_parent, b"blocked").unwrap();

        assert!(context.bind_user(7).is_err());
        assert_eq!(context.user_id, None);
    }

    #[test]
    fn concurrent_context_updates_fail_instead_of_overwriting() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("context.json");
        let mut first = SessionContext::load(Some(path.clone()), "http://one/api").unwrap();
        let mut stale = SessionContext::load(Some(path), "http://one/api").unwrap();

        first.set(Some(1), None, None).unwrap();
        assert!(stale.set(Some(2), None, None).is_err());
        assert_eq!(stale.project_id, None);
    }
}
