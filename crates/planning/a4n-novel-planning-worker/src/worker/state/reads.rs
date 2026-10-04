use super::super::errors::{shooting_script_error, source_error, visual_plan_error};
use super::super::helpers::*;
use crate::*;

impl WorkerState {
    pub(in crate::worker) async fn current(
        &self,
        request: WorkerEnvelope<ChapterShootingScriptSourceInput>,
    ) -> Result<Option<ChapterShootingScriptView>, WorkerFailure> {
        validate_request(&request, GET_CURRENT_SHOOTING_SCRIPT_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let current = self.current_record(&request.payload).await?;
        match current {
            Some(record) => self.shooting_script_view(record).await.map(Some),
            None => Ok(None),
        }
    }

    pub(in crate::worker) async fn unit_progress(
        &self,
        request: WorkerEnvelope<ChapterShootingScriptSourceInput>,
    ) -> Result<Option<ChapterShootingScriptUnitProgressView>, WorkerFailure> {
        validate_request(&request, GET_SHOOTING_SCRIPT_UNIT_PROGRESS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let source = resolve_source(
            self,
            request.payload.project_id,
            request.payload.source_chapter_id,
        )
        .await?;
        a4n_db::shooting_script::chapter_shooting_script_progress_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(shooting_script_error)
    }

    pub(in crate::worker) async fn shooting_script_view(
        &self,
        record: a4n_db::shooting_script::ParsedChapterShootingScript,
    ) -> Result<ChapterShootingScriptView, WorkerFailure> {
        let plan_sync = a4n_db::shooting_script::chapter_shooting_script_plan_sync(
            self.database.connection(),
            &record,
        )
        .await
        .map_err(shooting_script_error)?;
        Ok(ChapterShootingScriptView {
            record: record_summary(&record),
            script: record.script,
            plan_sync,
        })
    }

    pub(in crate::worker) async fn current_record(
        &self,
        input: &ChapterShootingScriptSourceInput,
    ) -> Result<Option<a4n_db::shooting_script::ParsedChapterShootingScript>, WorkerFailure> {
        let source = a4n_db::project::resolve_video_chapter_source(
            self.database.connection(),
            input.project_id,
            input.source_chapter_id,
        )
        .await
        .map_err(source_error)?;
        let existing = a4n_db::shooting_script::chapter_shooting_script_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(shooting_script_error)?;
        let confirmed = a4n_db::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(visual_plan_error)?;
        let Some(confirmed) = confirmed else {
            return Ok(None);
        };
        if input
            .visual_plan_id
            .is_some_and(|requested| requested != confirmed.record.id)
        {
            return Err(WorkerFailure::invalid_request(format!(
                "visual_plan_id 不是当前正文的 confirmed 方案: requested={:?}, current={}",
                input.visual_plan_id, confirmed.record.id
            )));
        }
        Ok(existing.filter(|record| {
            record.record.visual_plan_id == Some(confirmed.record.id)
                && record.record.visual_plan_version == Some(confirmed.record.plan_version)
                && record.record.visual_plan_sha256.as_deref()
                    == Some(confirmed.record.plan_sha256.as_str())
        }))
    }
}

fn record_summary(
    record: &a4n_db::shooting_script::ParsedChapterShootingScript,
) -> ChapterShootingScriptRecordSummary {
    a4n_db::shooting_script::chapter_shooting_script_record_summary(record)
}
