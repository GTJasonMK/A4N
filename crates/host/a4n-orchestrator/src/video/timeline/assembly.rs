impl OrchestratorContext {
    pub(crate) async fn current_video_timeline_snapshot(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<a4n_db::video::VideoTimelineSnapshot, AppError> {
        let timeline = self
            .ensure_chapter_video_timeline(
                project_id,
                chapter_id,
                chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
            )
            .await?;
        let (keys_result, snapshot_result) = futures::join!(
            self.native_keys_for_current_plan(project_id, chapter_id),
            self.chapter_video_timeline_snapshot(timeline.id),
        );
        let keys = keys_result?;
        let snapshot = snapshot_result?;
        match keys {
            Some(keys) if native_timeline_needs_reconcile(&snapshot, &keys)? => {
                self.reconcile_native_video_timeline(timeline.id, &keys)
                    .await
            }
            _ => Ok(snapshot),
        }
    }

    pub async fn get_video_timeline(
        &self,
        user_id: i64,
        input: GetVideoTimelineInput,
    ) -> Result<VideoTimelineSummary, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.project_chapter_video_source(
            input.project_id,
            input.chapter_id,
            "读取章节视频时间线",
            false,
        )
        .await?;
        let snapshot = self
            .current_video_timeline_snapshot(input.project_id, input.chapter_id)
            .await?;
        self.timeline_summary(input.project_id, input.chapter_id, snapshot)
            .await
    }

    pub async fn commit_inserted_segment_draft(
        &self,
        user_id: i64,
        input: CommitInsertedSegmentDraftInput,
    ) -> Result<VideoTimelineSummary, AppError> {
        let run = self.workflow_run_model_by_id(input.workflow_run_id).await?;
        validate_inserted_workflow_scope(&run, &input)?;
        let project_id = run.project_id.ok_or_else(|| AppError::InvalidState {
            current: "inserted_segment_workflow_missing_project".to_string(),
            operation: "提交插入片段草稿".to_string(),
        })?;
        let chapter_id = run.chapter_id.ok_or_else(|| AppError::InvalidState {
            current: "inserted_segment_workflow_missing_chapter".to_string(),
            operation: "提交插入片段草稿".to_string(),
        })?;
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .project_chapter_video_source(project_id, chapter_id, "提交插入片段草稿", true)
            .await?;
        let snapshot = self
            .chapter_video_timeline_snapshot(input.timeline_id)
            .await?;
        if snapshot.timeline.project_id != project_id || snapshot.timeline.chapter_id != chapter_id
        {
            return Err(AppError::InvalidArgument(
                "插入片段草稿与时间线不属于同一项目章节".to_string(),
            ));
        }
        if snapshot.timeline.workflow_mode
            != chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
        {
            return Err(AppError::InvalidArgument(
                "插入片段只支持当前共享片段规划时间线".to_string(),
            ));
        }
        if let Some(existing) = a4n_db::video::inserted_timeline_item_by_workflow_run(
            &snapshot.items,
            input.workflow_run_id,
        ) {
            if existing.status == a4n_db::video::TIMELINE_DELETED_STATUS {
                return Err(AppError::InvalidState {
                    current: "inserted_segment_already_deleted".to_string(),
                    operation: "重复提交插入片段草稿".to_string(),
                });
            }
            return self
                .timeline_summary(project_id, chapter_id, snapshot)
                .await;
        }
        if snapshot.timeline.revision != input.expected_timeline_revision {
            return Err(timeline_revision_conflict(
                snapshot.timeline.revision,
                input.expected_timeline_revision,
            ));
        }

        let input_fingerprint_json =
            run.input_fingerprint_json
                .as_ref()
                .ok_or_else(|| AppError::InvalidState {
                    current: "inserted_segment_workflow_missing_fingerprint".to_string(),
                    operation: "提交插入片段草稿".to_string(),
                })?;
        let fingerprint = inserted_workflow_fingerprint(&run, input_fingerprint_json)?;
        if fingerprint.timeline_id != input.timeline_id
            || fingerprint.timeline_revision != input.expected_timeline_revision
        {
            return Err(AppError::InvalidArgument(
                "插入片段提交参数与草稿生成时的时间线版本不一致".to_string(),
            ));
        }
        let source_hash = source_text_hash(&source.source.content);
        if fingerprint.source_content_sha256 != source_hash {
            return Err(AppError::InvalidState {
                current: "inserted_segment_source_changed".to_string(),
                operation: "章节正文已变化，请重新生成插入片段描述".to_string(),
            });
        }
        let neighbor_context = inserted_segment_neighbor_context(
            self,
            project_id,
            chapter_id,
            &snapshot,
            fingerprint.gap.previous_item_id,
            fingerprint.gap.next_item_id,
        )
        .await?;
        if fingerprint.context_sha256 != neighbor_context.sha256 {
            return Err(AppError::InvalidState {
                current: "inserted_segment_neighbor_context_changed".to_string(),
                operation: "相邻片段、采用视频或尾帧已变化，请重新生成插入片段描述".to_string(),
            });
        }
        let checkpoint = self
            .load_completed_llm_call_checkpoint(
                run.id,
                INSERTED_SEGMENT_CHECKPOINT_KEY,
                input_fingerprint_json,
            )
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "inserted_segment_draft_checkpoint_missing".to_string(),
                operation: "提交插入片段草稿".to_string(),
            })?;
        let prompt_language =
            PromptOutputLanguage::from_code(&fingerprint.payload.prompt_output_language)?;
        let draft = InsertedSegmentDraft::parse_and_validate(
            &checkpoint.response_content,
            prompt_language,
        )?;
        let reparsed_output = serde_json::to_value(&draft)?;
        ensure_checkpoint_parsed_output_matches(
            &checkpoint.parsed_output,
            &reparsed_output,
            "插入片段草稿",
        )?;

        let gap = VideoTimelineGap {
            previous_item_id: fingerprint.gap.previous_item_id,
            next_item_id: fingerprint.gap.next_item_id,
        };
        let order = a4n_db::video::active_timeline_order(&snapshot).map_err(|error| {
            AppError::InvalidState {
                current: error.to_string(),
                operation: "提交插入片段草稿".to_string(),
            }
        })?;
        let insertion_index = timeline_gap_index(&order, gap)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let segment_number =
            i32::try_from(insertion_index + 1).map_err(|_| AppError::InvalidState {
                current: "inserted_segment_gap_unreachable".to_string(),
                operation: "提交插入片段草稿".to_string(),
            })?;
        let recipe = inserted_segment_recipe(&fingerprint, segment_number, prompt_language)?;
        let provider_reference_ids = recipe.provider_reference_image_ids();
        if !provider_reference_ids.is_empty() {
            self.validated_video_reference_images(
                project_id,
                chapter_id,
                Some(&json!(provider_reference_ids)),
            )
            .await?;
        }
        validate_inserted_continuity_source(self, project_id, chapter_id, &recipe).await?;
        let segment_hash = source_text_hash(&serde_json::to_string(&json!({
            "workflow_run_id": run.id,
            "fingerprint": run.input_fingerprint_json,
            "draft": draft,
        }))?);
        let execution_identity = json!({
            "contract_epoch": VIDEO_EXECUTION_CONTRACT_EPOCH,
            "source_content_sha256": source_hash,
            "segment_sha256": segment_hash,
        });
        let metadata = json!({
            "contract_version": "inserted_segment_v1",
            "source_prompt_type": INSERTED_SEGMENT_SOURCE_PROMPT_TYPE,
            "source": {
                "content_sha256": source_hash,
                "content_chars": source.source.content.chars().count(),
            },
            "prompt_output_language": prompt_language.as_str(),
            "segment_recipe": recipe,
            "video_execution_identity": execution_identity,
        });
        self.commit_inserted_video_timeline_item(a4n_db::video::CommitInsertedTimelineItemInput {
            timeline_id: input.timeline_id,
            expected_revision: input.expected_timeline_revision,
            gap,
            prompt: a4n_domain::video_prompt::ChapterVideoPromptRecordFields {
                project_id,
                chapter_id,
                style: Some(fingerprint.payload.style.trim().to_string()),
                prompt_type: chapter_video_prompt::PROMPT_TYPE_VIDEO.to_string(),
                workflow_mode: recipe.workflow_mode.as_str().to_string(),
                shot_number: segment_number,
                title: Some(draft.title.trim().to_string()),
                scene_summary: Some(draft.scene_summary.trim().to_string()),
                visual_prompt: draft.visual_prompt.trim().to_string(),
                motion_prompt: Some(draft.motion_prompt.trim().to_string()),
                negative_prompt: Some(draft.negative_prompt.trim().to_string()),
                duration_secs: recipe.duration_secs,
                aspect_ratio: Some(recipe.aspect_ratio.clone()),
                reference_image_ids_json: Some(json!(provider_reference_ids)),
                metadata_json: Some(metadata),
            },
            workflow_run_id: run.id,
        })
        .await?;
        self.timeline_summary(
            project_id,
            chapter_id,
            self.chapter_video_timeline_snapshot(input.timeline_id)
                .await?,
        )
        .await
    }

    pub async fn move_video_timeline_item(
        &self,
        user_id: i64,
        input: MoveInsertedSegmentInput,
    ) -> Result<VideoTimelineSummary, AppError> {
        let before = self
            .chapter_video_timeline_snapshot(input.timeline_id)
            .await?;
        self.verify_project_permission(
            before.timeline.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        self.move_inserted_video_timeline_item(
            input.timeline_id,
            input.expected_timeline_revision,
            input.timeline_item_id,
            VideoTimelineGap {
                previous_item_id: input.previous_item_id,
                next_item_id: input.next_item_id,
            },
        )
        .await?;
        self.timeline_summary(
            before.timeline.project_id,
            before.timeline.chapter_id,
            self.chapter_video_timeline_snapshot(input.timeline_id)
                .await?,
        )
        .await
    }

    pub async fn delete_video_timeline_item(
        &self,
        user_id: i64,
        input: DeleteInsertedSegmentInput,
    ) -> Result<VideoTimelineSummary, AppError> {
        let before = self
            .chapter_video_timeline_snapshot(input.timeline_id)
            .await?;
        self.verify_project_permission(
            before.timeline.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        self.delete_inserted_video_timeline_item(
            input.timeline_id,
            input.expected_timeline_revision,
            input.timeline_item_id,
        )
        .await?;
        self.timeline_summary(
            before.timeline.project_id,
            before.timeline.chapter_id,
            self.chapter_video_timeline_snapshot(input.timeline_id)
                .await?,
        )
        .await
    }

    async fn native_keys_for_current_plan(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Option<Vec<NativeVideoTimelineKey>>, AppError> {
        let Some(plan) = self
            .current_shared_segment_plan_optional(project_id, chapter_id)
            .await?
        else {
            // 规划生成/切换期间不能把已有 native 项误判为删除。
            return Ok(None);
        };
        let segments = shared_plan_segments(&plan)?;
        let keys = segments
            .into_iter()
            .map(|segment| {
                NativeVideoTimelineKey::new(segment.source_shot_id, segment.segment_index_in_shot)
                    .map_err(AppError::InvalidArgument)
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(Some(keys))
    }

    async fn timeline_summary(
        &self,
        project_id: i64,
        chapter_id: i64,
        snapshot: a4n_db::video::VideoTimelineSnapshot,
    ) -> Result<VideoTimelineSummary, AppError> {
        let (prompts_result, videos_result, shared_plan_result) = futures::join!(
            self.chapter_video_prompt_timeline_records_for_chapter(project_id, chapter_id),
            self.generated_videos_for_chapter(project_id, chapter_id),
            self.current_shared_segment_plan_optional(project_id, chapter_id),
        );
        let prompts = prompts_result?;
        let videos = videos_result?;
        let shared_plan = shared_plan_result?;
        let segments = shared_plan
            .as_ref()
            .map(shared_plan_segments)
            .transpose()?
            .unwrap_or_default();
        let recipes = match shared_plan.as_ref() {
            Some(plan) => {
                let style = plan
                    .style
                    .as_deref()
                    .map(str::trim)
                    .filter(|style| !style.is_empty())
                    .ok_or_else(|| {
                        AppError::InvalidArgument("当前共享片段规划缺少 style".to_string())
                    })?;
                video_segment_recipes_for_plan_from_metadata(plan.metadata_json.as_ref(), style)
                    .map_err(AppError::InvalidArgument)?
            }
            None => Vec::new(),
        };
        let order = a4n_db::video::active_timeline_order(&snapshot).map_err(|error| {
            AppError::InvalidState {
                current: error.to_string(),
                operation: "生成章节视频时间线响应".to_string(),
            }
        })?;
        let by_id = snapshot
            .items
            .iter()
            .map(|item| (item.id, item))
            .collect::<HashMap<_, _>>();
        let prompts_by_id = prompts.iter().fold(
            HashMap::with_capacity(prompts.len()),
            |mut by_id, prompt| {
                by_id.entry(prompt.id).or_insert(prompt);
                by_id
            },
        );
        let videos_by_id =
            videos
                .iter()
                .fold(HashMap::with_capacity(videos.len()), |mut by_id, video| {
                    by_id.entry(video.id).or_insert(video);
                    by_id
                });
        let segments_by_identity = segments.iter().fold(
            HashMap::with_capacity(segments.len()),
            |mut by_identity, segment| {
                by_identity
                    .entry((
                        segment.source_shot_id.as_str(),
                        segment.segment_index_in_shot,
                    ))
                    .or_insert(segment);
                by_identity
            },
        );
        let recipes_by_number = recipes.iter().fold(
            HashMap::with_capacity(recipes.len()),
            |mut by_number, recipe| {
                by_number.entry(recipe.segment_number).or_insert(recipe);
                by_number
            },
        );
        let mut items = Vec::with_capacity(order.len());
        for (index, id) in order.into_iter().enumerate() {
            let item = by_id.get(&id).ok_or_else(|| AppError::InvalidState {
                current: "timeline_item_missing_from_snapshot".to_string(),
                operation: format!("生成章节视频时间线项 {id}"),
            })?;
            let display_index = i32::try_from(index + 1)
                .map_err(|_| AppError::InvalidArgument("时间线片段数量超出范围".to_string()))?;
            let prompt =
                item.insert_plan_prompt_id
                    .map(|prompt_id| {
                        prompts_by_id.get(&prompt_id).copied().ok_or_else(|| {
                            AppError::InvalidState {
                                current: "inserted_timeline_prompt_missing".to_string(),
                                operation: format!("读取插入时间线项 {id}"),
                            }
                        })
                    })
                    .transpose()?;
            let kind = match item.item_kind.as_str() {
                a4n_db::video::TIMELINE_INSERTED_KIND => ApiItemKind::Inserted,
                a4n_db::video::TIMELINE_NATIVE_KIND => ApiItemKind::Native,
                other => {
                    return Err(AppError::InvalidState {
                        current: format!("unknown_timeline_item_kind:{other}"),
                        operation: format!("生成章节视频时间线项 {id}"),
                    });
                }
            };
            let status = match item.status.as_str() {
                a4n_db::video::TIMELINE_NEEDS_REANCHOR_STATUS => ApiItemStatus::NeedsReanchor,
                a4n_db::video::TIMELINE_ACTIVE_STATUS => ApiItemStatus::Active,
                other => {
                    return Err(AppError::InvalidState {
                        current: format!("inactive_timeline_item_in_order:{other}"),
                        operation: format!("生成章节视频时间线项 {id}"),
                    });
                }
            };
            let native_segment = if kind == ApiItemKind::Native {
                shared_plan
                    .as_ref()
                    .map(|_| {
                        item.native_source_shot_id
                            .as_deref()
                            .zip(item.native_segment_index_in_shot)
                            .and_then(|identity| segments_by_identity.get(&identity).copied())
                            .ok_or_else(|| AppError::InvalidState {
                                current: "native_timeline_item_not_in_current_plan".to_string(),
                                operation: format!("生成章节视频时间线项 {id}"),
                            })
                    })
                    .transpose()?
            } else {
                None
            };
            let selected_video_id = match kind {
                ApiItemKind::Inserted => prompt.and_then(|row| row.generated_video_id),
                ApiItemKind::Native => native_segment.and_then(|segment| {
                    recipes_by_number
                        .get(&segment.segment_number)
                        .copied()
                        .and_then(|recipe| recipe.adopted_video_id)
                }),
            };
            let generated_video_id = selected_video_id
                .map(|video_id| {
                    let video = videos_by_id.get(&video_id).copied().ok_or_else(|| {
                        AppError::InvalidState {
                            current: format!("timeline_selected_video_missing:{video_id}"),
                            operation: format!("生成章节视频时间线项 {id}"),
                        }
                    })?;
                    if video.project_id != project_id
                        || video.chapter_id != Some(chapter_id)
                        || video.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
                        || !video_matches_timeline_item(video, item)
                    {
                        return Err(AppError::InvalidState {
                            current: format!("timeline_selected_video_invalid:{video_id}"),
                            operation: format!("生成章节视频时间线项 {id}"),
                        });
                    }
                    Ok(video_id)
                })
                .transpose()?;
            items.push(VideoTimelineItemSummary {
                id: item.id,
                kind,
                status,
                display_index,
                segment_number: native_segment.map(|segment| segment.segment_number),
                source_shot_id: item.native_source_shot_id.clone(),
                plan_prompt_id: item
                    .insert_plan_prompt_id
                    .or_else(|| shared_plan.as_ref().map(|row| row.id)),
                title: prompt
                    .and_then(|row| row.title.clone())
                    .or_else(|| native_segment.map(|segment| segment.title.clone())),
                generated_video_id,
            });
        }
        Ok(VideoTimelineSummary {
            id: snapshot.timeline.id,
            revision: snapshot.timeline.revision,
            items,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct VideoTimelineAssemblyOrder {
    pub(crate) display_index_by_item_id: HashMap<i64, i32>,
    pub(crate) native_item_id_by_identity: HashMap<(String, i32), i64>,
}

impl VideoTimelineAssemblyOrder {
    pub(crate) fn display_index_for_video(&self, video: &generated_video::Model) -> Option<i32> {
        let item_id = match video.timeline_item_id {
            Some(item_id) => item_id,
            None => {
                let identity = video
                    .metadata_json
                    .as_ref()?
                    .get("video_execution_identity")?;
                let source_shot_id = identity.get("source_shot_id")?.as_str()?;
                let segment_index = identity
                    .get("segment_index_in_shot")?
                    .as_i64()
                    .and_then(|value| i32::try_from(value).ok())?;
                self.native_item_id_by_identity
                    .get(&(source_shot_id.to_string(), segment_index))?
                    .to_owned()
            }
        };
        self.display_index_by_item_id.get(&item_id).copied()
    }
}

pub(crate) fn video_timeline_assembly_order_from_snapshot(
    snapshot: &a4n_db::video::VideoTimelineSnapshot,
) -> Result<VideoTimelineAssemblyOrder, AppError> {
    let order =
        a4n_db::video::active_timeline_order(snapshot).map_err(|error| AppError::InvalidState {
            current: error.to_string(),
            operation: "读取成片时间线顺序".to_string(),
        })?;
    let by_id = snapshot
        .items
        .iter()
        .map(|item| (item.id, item))
        .collect::<HashMap<_, _>>();
    let mut result = VideoTimelineAssemblyOrder::default();
    for (index, item_id) in order.into_iter().enumerate() {
        let item = by_id.get(&item_id).ok_or_else(|| AppError::InvalidState {
            current: "timeline_item_missing_from_snapshot".to_string(),
            operation: format!("读取成片时间线项 {item_id}"),
        })?;
        if item.status != a4n_db::video::TIMELINE_ACTIVE_STATUS {
            continue;
        }
        let display_index = i32::try_from(index + 1)
            .map_err(|_| AppError::InvalidArgument("时间线片段数量超出范围".to_string()))?;
        result
            .display_index_by_item_id
            .insert(item_id, display_index);
        if item.item_kind == a4n_db::video::TIMELINE_NATIVE_KIND
            && let (Some(source_shot_id), Some(segment_index_in_shot)) = (
                item.native_source_shot_id.clone(),
                item.native_segment_index_in_shot,
            )
        {
            result
                .native_item_id_by_identity
                .insert((source_shot_id, segment_index_in_shot), item_id);
        }
    }
    Ok(result)
}

pub(crate) fn video_matches_timeline_item(
    video: &a4n_db_models::models::ai::generated_video::Model,
    item: &a4n_db_models::models::chapter::chapter_video_timeline_item::Model,
) -> bool {
    let identity = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("video_execution_identity"));
    match item.item_kind.as_str() {
        a4n_db::video::TIMELINE_INSERTED_KIND => video.timeline_item_id == Some(item.id),
        a4n_db::video::TIMELINE_NATIVE_KIND => {
            video.timeline_item_id.is_none()
                && identity
                    .and_then(|identity| identity.get("source_shot_id"))
                    .and_then(Value::as_str)
                    == item.native_source_shot_id.as_deref()
                && identity
                    .and_then(|identity| identity.get("segment_index_in_shot"))
                    .and_then(Value::as_i64)
                    .and_then(|value| i32::try_from(value).ok())
                    == item.native_segment_index_in_shot
        }
        _ => false,
    }
}

pub(crate) async fn inserted_segment_neighbor_context(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    snapshot: &a4n_db::video::VideoTimelineSnapshot,
    previous_item_id: Option<i64>,
    next_item_id: Option<i64>,
) -> Result<InsertedSegmentNeighborContext, AppError> {
    let plan = service
        .current_shared_segment_plan(project_id, chapter_id)
        .await?;
    let plan_style = plan
        .style
        .as_deref()
        .map(str::trim)
        .filter(|style| !style.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("当前共享片段规划缺少 style".to_string()))?;
    let segments = shared_plan_segments(&plan)?;
    let recipes =
        video_segment_recipes_for_plan_from_metadata(plan.metadata_json.as_ref(), plan_style)
            .map_err(AppError::InvalidArgument)?;
    let (prompts_result, videos_result, images_result) = futures::join!(
        service.chapter_video_prompt_timeline_records_for_chapter(project_id, chapter_id),
        service.generated_videos_for_chapter(project_id, chapter_id),
        service.generated_images_for_video_prompt_references(project_id, chapter_id),
    );
    let prompts = prompts_result?;
    let videos = videos_result?;
    let images = images_result?;
    let previous = timeline_item_context(
        previous_item_id,
        &snapshot.items,
        &prompts,
        &segments,
        &recipes,
        &videos,
        &images,
    )?;
    let next = timeline_item_context(
        next_item_id,
        &snapshot.items,
        &prompts,
        &segments,
        &recipes,
        &videos,
        &images,
    )?;
    let serialized = serde_json::to_string(&json!({
        "previous_item": previous,
        "next_item": next,
    }))?;
    Ok(InsertedSegmentNeighborContext {
        previous,
        next,
        sha256: source_text_hash(&serialized),
    })
}
