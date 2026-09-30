use super::super::errors::{shooting_script_error, source_error, visual_plan_error};
use super::super::helpers::*;
use crate::*;

impl WorkerState {
    pub(in crate::worker) async fn edit(
        &self,
        request: WorkerEnvelope<EditChapterShootingScriptInput>,
    ) -> Result<EditedChapterShootingScriptView, WorkerFailure> {
        validate_request(&request, EDIT_SHOOTING_SCRIPT_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_target_revision(&request, request.payload.expected_revision)?;
        let user_id = user_id(&request)?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let current = self
            .current_record(&ChapterShootingScriptSourceInput {
                project_id: request.payload.project_id,
                source_chapter_id: request.payload.source_chapter_id,
                visual_plan_id: None,
            })
            .await?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!(
                    "项目 {} 的章节 {} 未找到与当前正文版本匹配的结构化拍摄剧本；请先生成",
                    request.payload.project_id, request.payload.source_chapter_id
                ))
            })?;
        ensure_record_id(current.record.id, request.payload.record_id)?;
        let outcome = a4n_db::shooting_script::edit_chapter_shooting_script_and_invalidate(
            self.database.connection(),
            a4n_domain::shooting_script::EditStoredChapterShootingScriptInput {
                record_id: request.payload.record_id,
                expected_revision: request.payload.expected_revision,
                script: request.payload.script,
                rebind_shot_ids: request.payload.rebind_shot_ids,
            },
        )
        .await
        .map_err(shooting_script_error)?;
        Ok(EditedChapterShootingScriptView {
            shooting_script: self.shooting_script_view(outcome.shooting_script).await?,
            invalidated_project_ids: outcome.invalidated_project_ids,
        })
    }

    pub(in crate::worker) async fn edit_progress(
        &self,
        request: WorkerEnvelope<EditChapterShootingScriptProgressInput>,
    ) -> Result<EditedChapterShootingScriptProgressView, WorkerFailure> {
        validate_request(&request, EDIT_SHOOTING_SCRIPT_PROGRESS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        a4n_db::shooting_script::edit_chapter_shooting_script_progress(
            self.database.connection(),
            request.payload,
        )
        .await
        .map_err(shooting_script_error)
    }

    pub(in crate::worker) async fn publish(
        &self,
        request: WorkerEnvelope<PublishChapterShootingScriptInput>,
    ) -> Result<ChapterShootingScriptView, WorkerFailure> {
        validate_request(&request, PUBLISH_SHOOTING_SCRIPT_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_optional_target_revision(&request, request.payload.expected_revision)?;
        let user_id = user_id(&request)?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;

        let source = a4n_db::project::resolve_video_chapter_source(
            self.database.connection(),
            request.payload.project_id,
            request.payload.source_chapter_id,
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
        .map_err(visual_plan_error)?
        .ok_or_else(|| WorkerFailure::not_found("当前正文缺少 confirmed 导演编排方案"))?;
        if confirmed.record.id != request.payload.visual_plan_id
            || confirmed.record.plan_version != request.payload.visual_plan_version
            || confirmed.record.plan_sha256 != request.payload.visual_plan_sha256
        {
            return Err(WorkerFailure::conflict(
                "发布的拍摄剧本未绑定当前 confirmed 导演编排",
            ));
        }

        let visual_plan = a4n_db::shooting_script::ChapterVisualPlanIdentity {
            id: request.payload.visual_plan_id,
            version: request.payload.visual_plan_version,
            sha256: request.payload.visual_plan_sha256.clone(),
        };
        let record = match (existing, request.payload.expected_revision) {
            (Some(existing), Some(expected_revision)) => {
                if existing.record.revision != expected_revision {
                    return Err(WorkerFailure::conflict(format!(
                        "shooting_script_revision_conflict:id={},expected={expected_revision}",
                        existing.record.id
                    )));
                }
                a4n_db::shooting_script::update_chapter_shooting_script(
                    self.database.connection(),
                    a4n_db::shooting_script::UpdateChapterShootingScriptInput {
                        record_id: existing.record.id,
                        expected_revision,
                        input_fingerprint_sha256: request.payload.input_fingerprint_sha256,
                        visual_plan,
                        script: request.payload.script,
                        workflow_run_id: Some(request.payload.workflow_run_id),
                    },
                )
                .await
                .map_err(shooting_script_error)?
            }
            (None, None) => a4n_db::shooting_script::insert_chapter_shooting_script(
                self.database.connection(),
                a4n_db::shooting_script::InsertChapterShootingScriptInput {
                    source,
                    input_fingerprint_sha256: request.payload.input_fingerprint_sha256,
                    visual_plan,
                    script: request.payload.script,
                    workflow_run_id: Some(request.payload.workflow_run_id),
                },
            )
            .await
            .map_err(shooting_script_error)?,
            (Some(existing), None) => {
                return Err(WorkerFailure::conflict(format!(
                    "shooting_script_revision_conflict:id={},expected_revision_required",
                    existing.record.id
                )));
            }
            (None, Some(expected_revision)) => {
                return Err(WorkerFailure::not_found(format!(
                    "expected_revision={expected_revision} 对应的章节拍摄剧本不存在"
                )));
            }
        };
        self.shooting_script_view(record).await
    }

    pub(in crate::worker) async fn delete(
        &self,
        request: WorkerEnvelope<DeleteChapterShootingScriptInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, DELETE_SHOOTING_SCRIPT_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_target_revision(&request, request.payload.expected_revision)?;
        let user_id = user_id(&request)?;
        verify_family_permission(
            self.database.connection(),
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let current = self
            .current_record(&ChapterShootingScriptSourceInput {
                project_id: request.payload.project_id,
                source_chapter_id: request.payload.source_chapter_id,
                visual_plan_id: None,
            })
            .await?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!(
                    "项目 {} 的章节 {} 未找到与当前正文版本匹配的结构化拍摄剧本；请先生成",
                    request.payload.project_id, request.payload.source_chapter_id
                ))
            })?;
        ensure_record_id(current.record.id, request.payload.record_id)?;
        a4n_db::shooting_script::delete_chapter_shooting_script_and_invalidate(
            self.database.connection(),
            a4n_db::shooting_script::DeleteStoredChapterShootingScriptInput {
                record_id: request.payload.record_id,
                expected_revision: request.payload.expected_revision,
            },
        )
        .await
        .map_err(shooting_script_error)
    }
}

fn ensure_record_id(expected: i64, actual: i64) -> Result<(), WorkerFailure> {
    if expected != actual {
        return Err(WorkerFailure::invalid_request(format!(
            "拍摄剧本与当前章节来源不匹配: expected_record_id={expected}, actual_record_id={actual}"
        )));
    }
    Ok(())
}
