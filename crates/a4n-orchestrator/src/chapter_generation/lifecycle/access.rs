use crate::{OrchestratorContext, project::PROJECT_KIND_TEXT};

impl OrchestratorContext {
    pub async fn ensure_text_source_project(
        &self,
        project_id: i64,
        operation: &str,
    ) -> Result<(), String> {
        let project = self
            .project_by_id(project_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("项目不存在: {project_id}"))?;
        if project.project_kind != PROJECT_KIND_TEXT {
            return Err(format!("{operation}只能作用于文本项目"));
        }
        Ok(())
    }
}
