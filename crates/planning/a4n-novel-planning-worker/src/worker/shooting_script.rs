impl WorkerState {
    async fn current(
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

    async fn unit_progress(
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

    async fn edit(
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

    async fn edit_progress(
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

    async fn publish(
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

    async fn delete(
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

    async fn shooting_script_view(
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
            record: record_summary(record.record),
            script: record.script,
            plan_sync,
        })
    }

    async fn current_record(
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

fn ensure_record_id(expected: i64, actual: i64) -> Result<(), WorkerFailure> {
    if expected != actual {
        return Err(WorkerFailure::invalid_request(format!(
            "拍摄剧本与当前章节来源不匹配: expected_record_id={expected}, actual_record_id={actual}"
        )));
    }
    Ok(())
}

fn record_summary(
    record: a4n_db_models::models::chapter::chapter_shooting_script::Model,
) -> ChapterShootingScriptRecordSummary {
    ChapterShootingScriptRecordSummary {
        id: record.id,
        revision: record.revision,
        visual_plan_id: record.visual_plan_id,
        visual_plan_version: record.visual_plan_version,
        visual_plan_sha256: record.visual_plan_sha256,
    }
}

fn shooting_script_error(error: a4n_db::shooting_script::ShootingScriptDbError) -> WorkerFailure {
    match error {
        a4n_db::shooting_script::ShootingScriptDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::shooting_script::ShootingScriptDbError::InvalidState(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::shooting_script::ShootingScriptDbError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::shooting_script::ShootingScriptDbError::RevisionConflict {
            record_id,
            expected_revision,
        } => WorkerFailure::conflict(format!(
            "shooting_script_revision_conflict:id={record_id},expected={expected_revision}"
        )),
        a4n_db::shooting_script::ShootingScriptDbError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}
