impl WorkerState {
    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }

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
        let Some(record) = current else {
            return Ok(None);
        };
        let plan_sync = self.plan_sync_for_record(&request.payload, &record).await?;
        Ok(Some(ChapterShootingScriptView {
            record: record_summary(record.record),
            script: record.script,
            plan_sync,
        }))
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
        let runs = a4n_db::workflow::workflow_runs_by_project_kind(
            self.database.connection(),
            source.root_project_id,
            SHOOTING_SCRIPT_WORKFLOW_KIND,
        )
        .await
        .map_err(workflow_read_error)?;
        let Some(run) = runs.into_iter().find(|run| {
            run.chapter_id == Some(source.source_chapter_id)
                && run.can_resume
                && run.status != a4n_domain::workflow::WORKFLOW_STATUS_SUPERSEDED
        }) else {
            return Ok(None);
        };
        let Some(snapshot) = run
            .progress_json
            .as_ref()
            .and_then(|progress| progress.get("snapshot"))
        else {
            return Ok(None);
        };
        let mut checkpoint: ChapterShootingScriptUnitProgressCheckpoint =
            serde_json::from_value(snapshot.clone()).map_err(|error| {
                WorkerFailure::invalid_request(format!("章节拍摄剧本进度 snapshot 无效: {error}"))
            })?;
        if checkpoint.version != 1 {
            return Err(WorkerFailure::invalid_request(format!(
                "不支持的章节拍摄剧本进度 snapshot 版本: {}",
                checkpoint.version
            )));
        }
        if checkpoint.source_content_sha256 != source.content_sha256 {
            return Ok(None);
        }
        if checkpoint.progress.workflow_run_id != run.id
            || checkpoint.progress.completed_unit_count == 0
            || checkpoint.progress.completed_unit_count > checkpoint.progress.total_unit_count
        {
            return Err(WorkerFailure::invalid_request(
                "章节拍摄剧本进度 snapshot 的场景数量不一致",
            ));
        }
        let confirmed = a4n_db::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(visual_plan_error)?
        .ok_or_else(|| WorkerFailure::not_found("当前正文缺少 confirmed 导演编排方案"))?;
        checkpoint
            .progress
            .script
            .validate_partial(
                checkpoint.progress.completed_unit_count,
                checkpoint.progress.total_unit_count,
            )
            .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
        let Some(identity) = checkpoint.progress.script.visual_plan_source.as_mut() else {
            return Err(WorkerFailure::invalid_request(
                "章节拍摄剧本进度 snapshot 缺少视觉编排身份",
            ));
        };
        if identity.plan_sha256 != confirmed.record.plan_sha256
            || (source.source_snapshot_id.is_none()
                && (identity.plan_id != confirmed.record.id
                    || identity.plan_version != confirmed.record.plan_version))
        {
            return Ok(None);
        }
        // 与完整剧本的快照复制一致，使用快照自己的方案身份，不改写源工作流。
        identity.plan_id = confirmed.record.id;
        identity.plan_version = confirmed.record.plan_version;
        Ok(Some(checkpoint.progress))
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
            },
        )
        .await
        .map_err(shooting_script_error)?;
        let plan_sync = self
            .plan_sync_for_record(
                &ChapterShootingScriptSourceInput {
                    project_id: request.payload.project_id,
                    source_chapter_id: request.payload.source_chapter_id,
                    visual_plan_id: None,
                },
                &outcome.shooting_script,
            )
            .await?;
        Ok(EditedChapterShootingScriptView {
            shooting_script: ChapterShootingScriptView {
                record: record_summary(outcome.shooting_script.record),
                script: outcome.shooting_script.script,
                plan_sync,
            },
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

        let source = a4n_db::project::resolve_chapter_source(
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
        let plan_sync = self
            .plan_sync_for_record(
                &ChapterShootingScriptSourceInput {
                    project_id: request.payload.project_id,
                    source_chapter_id: request.payload.source_chapter_id,
                    visual_plan_id: Some(request.payload.visual_plan_id),
                },
                &record,
            )
            .await?;
        Ok(ChapterShootingScriptView {
            record: record_summary(record.record),
            script: record.script,
            plan_sync,
        })
    }

    async fn save_visual_plan_draft(
        &self,
        request: WorkerEnvelope<SaveChapterVisualPlanDraftInput>,
    ) -> Result<ChapterVisualPlanView, WorkerFailure> {
        validate_request(&request, SAVE_VISUAL_PLAN_DRAFT_OPERATION)?;
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
        let source = resolve_source(
            self,
            request.payload.project_id,
            request.payload.source_chapter_id,
        )
        .await?;
        let expected = match (request.payload.record_id, request.payload.expected_revision) {
            (None, None) => None,
            (Some(record_id), Some(revision)) if record_id > 0 && revision > 0 => {
                Some(a4n_domain::chapter_visual_plan::ExpectedVisualPlanDraft {
                    record_id,
                    revision,
                })
            }
            _ => {
                return Err(WorkerFailure::invalid_request(
                    "record_id 与 expected_revision 必须同时提供且大于 0",
                ));
            }
        };
        let record = a4n_db::chapter_visual_plan::save_chapter_visual_plan_draft(
            self.database.connection(),
            a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
                source,
                author_user_id: user_id,
                authoring_mode: request.payload.authoring_mode,
                expected,
                expected_source_hash: request.payload.expected_source_hash,
                plan: request.payload.plan,
            },
        )
        .await
        .map_err(visual_plan_error)?;
        Ok(visual_plan_view(record))
    }

    async fn confirm_visual_plan(
        &self,
        request: WorkerEnvelope<ChapterVisualPlanRecordInput>,
    ) -> Result<ConfirmedChapterVisualPlanView, WorkerFailure> {
        validate_request(&request, CONFIRM_VISUAL_PLAN_OPERATION)?;
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
        ensure_visual_plan_belongs_to_source(self, &request.payload).await?;
        let outcome = a4n_db::chapter_visual_plan::confirm_chapter_visual_plan(
            self.database.connection(),
            a4n_domain::chapter_visual_plan::ConfirmChapterVisualPlanInput {
                record_id: request.payload.record_id,
                expected_revision: request.payload.expected_revision,
            },
        )
        .await
        .map_err(visual_plan_error)?;
        Ok(ConfirmedChapterVisualPlanView {
            visual_plan: visual_plan_view(outcome.visual_plan),
            invalidated_project_ids: outcome.invalidated_project_ids,
            shooting_script_sync: outcome.shooting_script_sync,
        })
    }

    async fn discard_visual_plan(
        &self,
        request: WorkerEnvelope<ChapterVisualPlanRecordInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, DISCARD_VISUAL_PLAN_OPERATION)?;
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
        ensure_visual_plan_belongs_to_source(self, &request.payload).await?;
        a4n_db::chapter_visual_plan::discard_chapter_visual_plan_draft(
            self.database.connection(),
            a4n_domain::chapter_visual_plan::DiscardChapterVisualPlanDraftInput {
                record_id: request.payload.record_id,
                expected_revision: request.payload.expected_revision,
            },
        )
        .await
        .map_err(visual_plan_error)
    }

    async fn get_visual_plan(
        &self,
        request: WorkerEnvelope<ChapterVisualPlanSourceInput>,
    ) -> Result<ChapterVisualPlanSetView, WorkerFailure> {
        validate_request(&request, GET_VISUAL_PLAN_OPERATION)?;
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
        let plans = a4n_db::chapter_visual_plan::chapter_visual_plans_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(visual_plan_error)?;
        Ok(ChapterVisualPlanSetView {
            source: a4n_api_contract::chapter_visual_plan::ChapterVisualPlanSourceView {
                chapter_number: source.chapter_number,
                title: source.title,
                content: source.content,
                content_sha256: source.content_sha256,
            },
            starter_plan: a4n_domain::chapter_visual_plan::ChapterVisualPlanV3::starter(),
            draft: plans.draft.map(visual_plan_view),
            confirmed: plans.confirmed.map(visual_plan_view),
        })
    }

    async fn read_visual_plan(
        &self,
        request: WorkerEnvelope<ReadChapterVisualPlanInput>,
    ) -> Result<ChapterVisualPlanReadResult, WorkerFailure> {
        validate_request(&request, READ_VISUAL_PLAN_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_optional_target_revision(&request, request.payload.expected_revision)?;
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
        let plans = a4n_db::chapter_visual_plan::chapter_visual_plans_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(visual_plan_error)?;
        let current = if let Some(record_id) = request.payload.record_id {
            let plan = a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
                self.database.connection(),
                record_id,
            )
            .await
            .map_err(visual_plan_error)?;
            ensure_visual_plan_belongs_to_source_record(&plan, &source)?;
            if let Some(expected_revision) = request.payload.expected_revision
                && plan.record.revision != expected_revision
            {
                return Err(visual_plan_revision_conflict(
                    plan.record.id,
                    expected_revision,
                ));
            }
            Some(plan)
        } else {
            plans.draft.or(plans.confirmed)
        };
        let resource = current.as_ref().map(visual_plan_resource);
        let starter_plan = a4n_domain::chapter_visual_plan::ChapterVisualPlanV3::starter();
        let projection = match &request.payload.view {
            VisualPlanReadView::Source {
                offset,
                limit,
                expected_hash,
            } => {
                if expected_hash
                    .as_ref()
                    .is_some_and(|hash| hash != &source.content_sha256)
                {
                    return Err(WorkerFailure::conflict("原文版本已变化，请重新读取"));
                }
                let total_chars = source.content.chars().count();
                let (start, end) = page_range(*offset, *limit, total_chars, 2_000, 2_000)?;
                VisualPlanProjection::Source {
                    text: source
                        .content
                        .chars()
                        .skip(start)
                        .take(end - start)
                        .collect(),
                    offset: *offset,
                    total_chars,
                    next_offset: next_offset(end, total_chars)?,
                }
            }
            view => a4n_api_contract::chapter_visual_plan::project_visual_plan(
                current.as_ref().map_or(&starter_plan, |plan| &plan.plan),
                view,
            )
            .map_err(projection_error)?,
        };
        Ok(ChapterVisualPlanReadResult {
            chapter_id: source.source_chapter_id,
            chapter_number: source.chapter_number,
            resource,
            source_hash: source.content_sha256,
            projection,
        })
    }

    async fn validate_visual_plan(
        &self,
        request: WorkerEnvelope<ValidateChapterVisualPlanInput>,
    ) -> Result<ChapterVisualPlanValidationResult, WorkerFailure> {
        validate_request(&request, VALIDATE_VISUAL_PLAN_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        validate_target_revision(&request, request.payload.expected_revision)?;
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
        let plan = a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
            self.database.connection(),
            request.payload.record_id,
        )
        .await
        .map_err(visual_plan_error)?;
        ensure_visual_plan_belongs_to_source_record(&plan, &source)?;
        if plan.record.revision != request.payload.expected_revision {
            return Err(visual_plan_revision_conflict(
                plan.record.id,
                request.payload.expected_revision,
            ));
        }
        let result = match request.payload.mode {
            a4n_api_contract::chapter_visual_plan::VisualPlanValidationMode::Draft => {
                plan.plan.validate_draft()
            }
            a4n_api_contract::chapter_visual_plan::VisualPlanValidationMode::Confirmed => {
                plan.plan.validate_confirmed()
            }
        };
        let validation = a4n_api_contract::chapter_visual_plan::VisualPlanValidationResult {
            mode: request.payload.mode,
            valid: result.is_ok(),
            diagnostics: result
                .err()
                .into_iter()
                .map(
                    |error| a4n_api_contract::chapter_visual_plan::VisualPlanDiagnostic {
                        path: error.path,
                        message: error.message,
                    },
                )
                .collect(),
        };
        Ok(ChapterVisualPlanValidationResult {
            resource: visual_plan_resource(&plan),
            validation,
        })
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

    async fn current_record(
        &self,
        input: &ChapterShootingScriptSourceInput,
    ) -> Result<Option<a4n_db::shooting_script::ParsedChapterShootingScript>, WorkerFailure> {
        let source = a4n_db::project::resolve_chapter_source(
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

    async fn plan_sync_for_record(
        &self,
        input: &ChapterShootingScriptSourceInput,
        record: &a4n_db::shooting_script::ParsedChapterShootingScript,
    ) -> Result<a4n_domain::shooting_script::ShootingScriptPlanSyncV1, WorkerFailure> {
        let source = a4n_db::project::resolve_chapter_source(
            self.database.connection(),
            input.project_id,
            input.source_chapter_id,
        )
        .await
        .map_err(source_error)?;
        let confirmed = a4n_db::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(visual_plan_error)?
        .ok_or_else(|| WorkerFailure::not_found("当前正文缺少 confirmed 导演编排方案"))?;
        let director = a4n_domain::shooting_script::project_visual_plan_for_shooting_script(
            &confirmed.plan,
            &[],
        )
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
        Ok(record.script.plan_sync(&director))
    }
}
