use a4n_db_models::models::generation::workflow_run;
use chrono::NaiveDateTime;

pub struct WorkflowRunModelBuilder {
    model: workflow_run::Model,
}

impl WorkflowRunModelBuilder {
    pub fn new(
        id: i64,
        business_key: impl Into<String>,
        workflow_kind: impl Into<String>,
        status: impl Into<String>,
        timestamp: NaiveDateTime,
    ) -> Self {
        Self {
            model: workflow_run::Model {
                id,
                business_key: business_key.into(),
                workflow_kind: workflow_kind.into(),
                project_id: None,
                root_project_id: None,
                source_snapshot_id: None,
                scope_table: None,
                scope_id: None,
                chapter_id: None,
                mode: None,
                input_fingerprint_json: None,
                status: status.into(),
                current_step_key: None,
                progress_json: None,
                active_task_id: None,
                can_resume: false,
                resume_block_reason: None,
                error_message: None,
                created_at: timestamp,
                updated_at: timestamp,
                completed_at: None,
            },
        }
    }

    pub fn with_project_scope(
        mut self,
        project_id: Option<i64>,
        root_project_id: Option<i64>,
    ) -> Self {
        self.model.project_id = project_id;
        self.model.root_project_id = root_project_id;
        self
    }

    pub fn with_source_snapshot_id(mut self, source_snapshot_id: i64) -> Self {
        self.model.source_snapshot_id = Some(source_snapshot_id);
        self
    }

    pub fn with_scope(mut self, scope_table: impl Into<String>, scope_id: i64) -> Self {
        self.model.scope_table = Some(scope_table.into());
        self.model.scope_id = Some(scope_id);
        self
    }

    pub fn with_chapter_id(mut self, chapter_id: i64) -> Self {
        self.model.chapter_id = Some(chapter_id);
        self
    }

    pub fn with_mode(mut self, mode: impl Into<String>) -> Self {
        self.model.mode = Some(mode.into());
        self
    }

    pub fn with_input_fingerprint(mut self, input_fingerprint: serde_json::Value) -> Self {
        self.model.input_fingerprint_json = Some(input_fingerprint);
        self
    }

    pub fn with_current_step(mut self, current_step_key: impl Into<String>) -> Self {
        self.model.current_step_key = Some(current_step_key.into());
        self
    }

    pub fn with_progress(mut self, progress: serde_json::Value) -> Self {
        self.model.progress_json = Some(progress);
        self
    }

    pub fn with_active_task(mut self, active_task_id: impl Into<String>) -> Self {
        self.model.active_task_id = Some(active_task_id.into());
        self
    }

    pub fn with_resumability(
        mut self,
        can_resume: bool,
        resume_block_reason: Option<String>,
    ) -> Self {
        self.model.can_resume = can_resume;
        self.model.resume_block_reason = resume_block_reason;
        self
    }

    pub fn with_error_message(mut self, error_message: impl Into<String>) -> Self {
        self.model.error_message = Some(error_message.into());
        self
    }

    pub fn with_updated_at(mut self, updated_at: NaiveDateTime) -> Self {
        self.model.updated_at = updated_at;
        self
    }

    pub fn with_completed_at(mut self, completed_at: Option<NaiveDateTime>) -> Self {
        self.model.completed_at = completed_at;
        self
    }

    pub fn build(self) -> workflow_run::Model {
        self.model
    }
}

#[cfg(test)]
mod tests;
