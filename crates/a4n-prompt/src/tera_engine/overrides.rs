use tera::Tera;

use a4n_core::error::AppError;

use super::state::TeraPromptEngine;

impl TeraPromptEngine {
    pub(super) fn resolve_content(
        &self,
        name: &str,
        project_id: Option<i64>,
    ) -> Result<(String, bool), AppError> {
        let entry_content = self
            .entries
            .read()
            .get(name)
            .map(|entry| entry.content.clone())
            .ok_or_else(|| Self::prompt_not_found(name))?;
        if let Some(project_id) = project_id {
            Self::validate_override_project_id(project_id)?;
            if let Some(override_) = self.overrides.read().get(&(project_id, name.to_string())) {
                return Ok((override_.clone(), true));
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

    fn prepare_project_override(
        &self,
        project_id: i64,
        name: &str,
        content: String,
    ) -> Result<(i64, String, String), AppError> {
        let (project_id, name) = Self::normalize_override_identity(project_id, name)?;
        if content.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "提示词覆盖内容不能为空".to_string(),
            ));
        }
        if !self.entries.read().contains_key(&name) {
            return Err(Self::prompt_not_found(&name));
        }
        let mut tera = Tera::default();
        tera.add_raw_template(&name, &content)
            .map_err(|error| AppError::Internal(anyhow::anyhow!("模板编译失败: {error}")))?;
        Ok((project_id, name, content))
    }

    pub fn hydrate_project_overrides(
        &self,
        overrides: Vec<(i64, String, String)>,
    ) -> Result<(), AppError> {
        let prepared = overrides
            .into_iter()
            .map(|(project_id, name, content)| {
                self.prepare_project_override(project_id, &name, content)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut identities = std::collections::HashSet::new();
        for (project_id, name, _) in &prepared {
            if !identities.insert((*project_id, name.as_str())) {
                return Err(AppError::InvalidArgument(format!(
                    "提示词覆盖 hydration 包含重复 identity：project_id={project_id}, name={name}"
                )));
            }
        }
        let mut runtime_overrides = self.overrides.write();
        for (project_id, name, content) in prepared {
            runtime_overrides.insert((project_id, name), content);
        }
        Ok(())
    }

    pub fn replace_project_overrides(
        &self,
        project_id: i64,
        overrides: Vec<(String, String)>,
    ) -> Result<(), AppError> {
        let project_id = Self::validate_override_project_id(project_id)?;
        let prepared = overrides
            .into_iter()
            .map(|(name, content)| self.prepare_project_override(project_id, &name, content))
            .collect::<Result<Vec<_>, _>>()?;
        let mut names = std::collections::HashSet::new();
        for (_, name, _) in &prepared {
            if !names.insert(name.as_str()) {
                return Err(AppError::InvalidArgument(format!(
                    "提示词覆盖 hydration 包含重复 name：{name}"
                )));
            }
        }

        let mut runtime_overrides = self.overrides.write();
        runtime_overrides.retain(|(stored_project_id, _), _| *stored_project_id != project_id);
        for (_, name, content) in prepared {
            runtime_overrides.insert((project_id, name), content);
        }
        Ok(())
    }

    pub fn set_project_override(
        &self,
        project_id: i64,
        name: &str,
        content: String,
    ) -> Result<(), AppError> {
        let (project_id, name, content) =
            self.prepare_project_override(project_id, name, content)?;
        self.overrides.write().insert((project_id, name), content);
        Ok(())
    }

    pub fn remove_project_override(&self, project_id: i64, name: &str) -> Result<(), AppError> {
        let (project_id, name) = Self::normalize_override_identity(project_id, name)?;
        self.overrides.write().remove(&(project_id, name));
        Ok(())
    }
}
