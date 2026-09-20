//! assembly application ports。

use super::*;
use async_trait::async_trait;

#[async_trait]
impl VideoAssemblyPostprocessApplicationPort for crate::OrchestratorContext {
    async fn prepare_assembly_postprocess_edit(
        &self,
        push_video_id: i64,
        operation: AssemblyPostprocessOperation,
    ) -> Result<AssemblyPostprocessPlan, AppError> {
        self.prepare_assembly_postprocess_edit_validated(push_video_id, operation)
            .await
    }

    async fn prepare_assembly_postprocess_source(
        &self,
        plan: &AssemblyPostprocessPlan,
        source_video_id: Option<i64>,
        has_uploaded_source: bool,
    ) -> Result<AssemblyPostprocessSourcePlan, AppError> {
        self.prepare_assembly_postprocess_source_validated(
            plan,
            source_video_id,
            has_uploaded_source,
        )
        .await
    }

    async fn create_assembly_postprocess_edit_record(
        &self,
        plan: AssemblyPostprocessPlan,
        input: AssemblyPostprocessEditRecordInput,
    ) -> Result<PreparedAssemblyPostprocessEditRecord, AppError> {
        self.create_assembly_postprocess_edit_record_validated(plan, input)
            .await
    }
}

#[async_trait]
impl VideoAssemblyPlanningApplicationPort for crate::OrchestratorContext {
    async fn assembly_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        operation: &str,
    ) -> Result<AssemblyVideoRecords, AppError> {
        self.assembly_video_records_validated(project_id, chapter_id, workflow_mode, operation)
            .await
    }
}

#[async_trait]
impl VideoAssemblyPushApplicationPort for crate::OrchestratorContext {
    async fn create_assembly_push_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        clips: Vec<AssemblyPushClipInput>,
        end_video_id: Option<i64>,
    ) -> Result<PreparedAssemblyPushRecord, AppError> {
        self.create_assembly_push_record_validated(
            project_id,
            chapter_id,
            workflow_mode,
            clips,
            end_video_id,
        )
        .await
    }

    async fn update_assembly_push_status(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateAssemblyPushStatusInput,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let existing = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            existing.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(generated_video_summary(
            &self
                .update_assembly_push_status_validated(existing.id, input)
                .await?,
        ))
    }
}
