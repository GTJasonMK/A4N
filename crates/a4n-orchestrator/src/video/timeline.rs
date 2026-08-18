//! 章节视频时间线应用规则。

use a4n_api_contract::video::{
    CommitInsertedSegmentDraftInput, DeleteInsertedSegmentInput, GetVideoTimelineInput,
    MoveInsertedSegmentInput, VideoTimelineItemKind as ApiItemKind,
    VideoTimelineItemStatus as ApiItemStatus, VideoTimelineItemSummary, VideoTimelineSummary,
};
use a4n_core::{
    error::AppError,
    models::{
        ai::{generated_image, generated_video},
        chapter::{chapter_video_prompt, chapter_video_timeline_item},
    },
    prompt_language::PromptOutputLanguage,
    source_spans::source_text_hash,
    video_execution::{
        NativeVideoTimelineKey, VIDEO_EXECUTION_CONTRACT_EPOCH, VIDEO_SEGMENT_EXECUTION_RECIPE_V1,
        VideoSegmentExecutionRecipeV1, VideoSegmentGuidanceModeV1, VideoSegmentPlanV2,
        VideoSegmentWorkflowModeV1, VideoTimelineGap, timeline_gap_index,
        video_segment_recipes_for_plan_from_metadata,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::workflow::WORKFLOW_STATUS_COMPLETED;
use std::collections::HashMap;

use crate::{OrchestratorContext, structured_llm::ensure_checkpoint_parsed_output_matches};
use crate::collaboration::ProjectAccessRole;
use crate::video::prompt_json::InsertedSegmentDraft;

pub(crate) const INSERTED_SEGMENT_SOURCE_PROMPT_TYPE: &str = "inserted_segment";
pub(crate) const INSERTED_SEGMENT_WORKFLOW_KIND: &str = "video_inserted_segment";
pub(crate) const INSERTED_SEGMENT_CHECKPOINT_KEY: &str = "inserted_segment_draft";
pub(crate) const VIDEO_TIMELINE_SCOPE_TABLE: &str = "chapter_video_timeline";

pub(crate) struct InsertedSegmentNeighborContext {
    pub(crate) previous: Value,
    pub(crate) next: Value,
    pub(crate) sha256: String,
}

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
        let keys = self
            .native_keys_for_current_plan(project_id, chapter_id)
            .await?;
        let snapshot = self.chapter_video_timeline_snapshot(timeline.id).await?;
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

        let input_fingerprint_json = run.input_fingerprint_json.as_ref().ok_or_else(|| {
            AppError::InvalidState {
                current: "inserted_segment_workflow_missing_fingerprint".to_string(),
                operation: "提交插入片段草稿".to_string(),
            }
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
        let draft =
            InsertedSegmentDraft::parse_and_validate(&checkpoint.response_content, prompt_language)?;
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
            prompt: a4n_db::video::ChapterVideoPromptRecordFields {
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
            a4n_core::video_execution::VideoTimelineGap {
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
        let prompts = self
            .chapter_video_prompts_for_chapter(project_id, chapter_id)
            .await?;
        let videos = self
            .generated_videos_for_chapter(project_id, chapter_id)
            .await?;
        let shared_plan = self
            .current_shared_segment_plan_optional(project_id, chapter_id)
            .await?;
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
        let mut items = Vec::with_capacity(order.len());
        for (index, id) in order.into_iter().enumerate() {
            let item = by_id.get(&id).ok_or_else(|| AppError::InvalidState {
                current: "timeline_item_missing_from_snapshot".to_string(),
                operation: format!("生成章节视频时间线项 {id}"),
            })?;
            let display_index = i32::try_from(index + 1)
                .map_err(|_| AppError::InvalidArgument("时间线片段数量超出范围".to_string()))?;
            let prompt = item
                .insert_plan_prompt_id
                .map(|prompt_id| {
                    prompts
                        .iter()
                        .find(|row| row.id == prompt_id)
                        .ok_or_else(|| AppError::InvalidState {
                            current: "inserted_timeline_prompt_missing".to_string(),
                            operation: format!("读取插入时间线项 {id}"),
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
                        segments
                            .iter()
                            .find(|segment| {
                                Some(segment.source_shot_id.as_str())
                                    == item.native_source_shot_id.as_deref()
                                    && Some(segment.segment_index_in_shot)
                                        == item.native_segment_index_in_shot
                            })
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
                    recipes
                        .iter()
                        .find(|recipe| recipe.segment_number == segment.segment_number)
                        .and_then(|recipe| recipe.adopted_video_id)
                }),
            };
            let generated_video_id = selected_video_id
                .map(|video_id| {
                    let video = videos
                        .iter()
                        .find(|video| video.id == video_id)
                        .ok_or_else(|| AppError::InvalidState {
                            current: format!("timeline_selected_video_missing:{video_id}"),
                            operation: format!("生成章节视频时间线项 {id}"),
                        })?;
                    if video.project_id != project_id
                        || video.chapter_id != Some(chapter_id)
                        || video.status != generated_video::STATUS_COMPLETED
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
    video: &a4n_core::models::ai::generated_video::Model,
    item: &a4n_core::models::chapter::chapter_video_timeline_item::Model,
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
    let prompts = service
        .chapter_video_prompts_for_chapter(project_id, chapter_id)
        .await?;
    let videos = service
        .generated_videos_for_chapter(project_id, chapter_id)
        .await?;
    let images = service
        .generated_images_for_video_prompt_references(project_id, chapter_id)
        .await?;
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

fn timeline_item_context(
    item_id: Option<i64>,
    items: &[chapter_video_timeline_item::Model],
    prompts: &[chapter_video_prompt::Model],
    segments: &[VideoSegmentPlanV2],
    recipes: &[VideoSegmentExecutionRecipeV1],
    videos: &[generated_video::Model],
    images: &[generated_image::Model],
) -> Result<Value, AppError> {
    let Some(item_id) = item_id else {
        return Ok(Value::Null);
    };
    let item = items
        .iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| AppError::InvalidState {
            current: "timeline_gap_item_missing".to_string(),
            operation: format!("读取插入片段相邻项 {item_id}"),
        })?;
    if item.status != a4n_db::video::TIMELINE_ACTIVE_STATUS {
        return Err(AppError::InvalidState {
            current: format!("timeline_gap_item_inactive:{}", item.status),
            operation: format!("相邻时间线项 {item_id} 需要先重新定位"),
        });
    }
    let prompt = item
        .insert_plan_prompt_id
        .map(|id| {
            prompts
                .iter()
                .find(|prompt| prompt.id == id)
                .ok_or_else(|| AppError::InvalidState {
                    current: "inserted_timeline_prompt_missing".to_string(),
                    operation: format!("读取插入片段相邻项 {item_id}"),
                })
        })
        .transpose()?;
    let segment = if item.item_kind == a4n_db::video::TIMELINE_NATIVE_KIND {
        Some(
            segments
                .iter()
                .find(|segment| {
                    Some(segment.source_shot_id.as_str()) == item.native_source_shot_id.as_deref()
                        && Some(segment.segment_index_in_shot) == item.native_segment_index_in_shot
                })
                .ok_or_else(|| AppError::InvalidState {
                    current: "native_timeline_item_not_in_current_plan".to_string(),
                    operation: format!("读取插入片段相邻项 {item_id}"),
                })?,
        )
    } else {
        None
    };
    let selected_video_id = match item.item_kind.as_str() {
        a4n_db::video::TIMELINE_INSERTED_KIND => {
            prompt.and_then(|prompt| prompt.generated_video_id)
        }
        a4n_db::video::TIMELINE_NATIVE_KIND => segment.and_then(|segment| {
            recipes
                .iter()
                .find(|recipe| recipe.segment_number == segment.segment_number)
                .and_then(|recipe| recipe.adopted_video_id)
        }),
        other => {
            return Err(AppError::InvalidState {
                current: format!("unknown_timeline_item_kind:{other}"),
                operation: format!("读取插入片段相邻项 {item_id}"),
            });
        }
    };
    let video_id = current_timeline_item_video_id(item, selected_video_id, videos)?;
    let tail_frame_image_id = video_id.and_then(|video_id| {
        images
            .iter()
            .find(|image| crate::video::tail_frame_image_belongs_to_video(image, video_id))
            .map(|image| image.id)
    });
    Ok(json!({
        "id": item.id,
        "kind": item.item_kind,
        "title": prompt.and_then(|prompt| prompt.title.clone())
            .or_else(|| segment.map(|segment| segment.title.clone())),
        "scene_summary": prompt.and_then(|prompt| prompt.scene_summary.clone())
            .or_else(|| segment.map(|segment| segment.scene_summary.clone())),
        "visual_prompt": prompt.map(|prompt| prompt.visual_prompt.clone()),
        "motion_prompt": prompt.and_then(|prompt| prompt.motion_prompt.clone()),
        "segment": segment,
        "video": video_id.map(|video_id| json!({
            "video_id": video_id,
            "tail_frame_image_id": tail_frame_image_id,
        })),
    }))
}

pub(crate) fn current_timeline_item_video_id(
    item: &chapter_video_timeline_item::Model,
    current_video_id: Option<i64>,
    videos: &[generated_video::Model],
) -> Result<Option<i64>, AppError> {
    let Some(video_id) = current_video_id else {
        return Ok(None);
    };
    let video = videos
        .iter()
        .find(|video| video.id == video_id)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("timeline_selected_video_missing:{video_id}"),
            operation: format!("读取时间线项 {} 的当前视频", item.id),
        })?;
    if video.status != generated_video::STATUS_COMPLETED || !video_matches_timeline_item(video, item)
    {
        return Err(AppError::InvalidState {
            current: format!("timeline_selected_video_invalid:{video_id}"),
            operation: format!("读取时间线项 {} 的当前视频", item.id),
        });
    }
    Ok(Some(video_id))
}

pub(crate) fn shared_plan_segments(
    plan: &a4n_core::models::chapter::chapter_video_prompt::Model,
) -> Result<Vec<VideoSegmentPlanV2>, AppError> {
    let segments = serde_json::from_value::<Vec<VideoSegmentPlanV2>>(
        plan.metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("segments"))
            .cloned()
            .ok_or_else(|| {
                AppError::InvalidArgument("当前共享片段规划缺少 segments".to_string())
            })?,
    )
    .map_err(|error| AppError::InvalidArgument(format!("当前共享片段规划无效：{error}")))?;
    if segments.is_empty() {
        return Err(AppError::InvalidArgument(
            "当前共享片段规划不能是空规划".to_string(),
        ));
    }
    Ok(segments)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InsertedSegmentWorkflowFingerprint {
    schema_version: i32,
    workflow_kind: String,
    project_id: i64,
    chapter_id: i64,
    source_content_sha256: String,
    context_sha256: String,
    timeline_id: i64,
    timeline_revision: i64,
    gap: VideoTimelineGap,
    payload: InsertedSegmentPayload,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InsertedSegmentPayload {
    user_prompt: String,
    style: String,
    prompt_output_language: String,
    constraints: InsertedSegmentConstraints,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InsertedSegmentConstraints {
    selected_duration_secs: i32,
    duration_secs_options: Vec<i32>,
    aspect_ratio: String,
    guidance_mode: VideoSegmentGuidanceModeV1,
    continuity_source_video_id: Option<i64>,
    reference_image_ids: Vec<i64>,
}

fn validate_inserted_workflow_scope(
    run: &a4n_core::models::generation::workflow_run::Model,
    input: &CommitInsertedSegmentDraftInput,
) -> Result<(), AppError> {
    if run.workflow_kind != INSERTED_SEGMENT_WORKFLOW_KIND {
        return Err(AppError::InvalidArgument(format!(
            "workflow #{} 不是插入片段草稿",
            run.id
        )));
    }
    if run.status != WORKFLOW_STATUS_COMPLETED {
        return Err(AppError::InvalidState {
            current: run.status.clone(),
            operation: "提交尚未完成的插入片段草稿".to_string(),
        });
    }
    if run.scope_table.as_deref() != Some(VIDEO_TIMELINE_SCOPE_TABLE)
        || run.scope_id != Some(input.timeline_id)
    {
        return Err(AppError::InvalidArgument(
            "插入片段 workflow 不属于目标时间线".to_string(),
        ));
    }
    Ok(())
}

fn inserted_workflow_fingerprint(
    run: &a4n_core::models::generation::workflow_run::Model,
    value: &Value,
) -> Result<InsertedSegmentWorkflowFingerprint, AppError> {
    let fingerprint: InsertedSegmentWorkflowFingerprint =
        serde_json::from_value(value.clone()).map_err(|error| {
            AppError::InvalidArgument(format!("插入片段 workflow 指纹无效: {error}"))
        })?;
    if fingerprint.schema_version != 1
        || fingerprint.workflow_kind != INSERTED_SEGMENT_WORKFLOW_KIND
        || Some(fingerprint.project_id) != run.project_id
        || Some(fingerprint.chapter_id) != run.chapter_id
    {
        return Err(AppError::InvalidState {
            current: "inserted_segment_workflow_fingerprint_scope_mismatch".to_string(),
            operation: "提交插入片段草稿".to_string(),
        });
    }
    if fingerprint.source_content_sha256.trim().is_empty()
        || fingerprint.context_sha256.trim().is_empty()
        || fingerprint.payload.user_prompt.trim().is_empty()
    {
        return Err(AppError::InvalidState {
            current: "inserted_segment_workflow_fingerprint_incomplete".to_string(),
            operation: "提交插入片段草稿".to_string(),
        });
    }
    Ok(fingerprint)
}

fn inserted_segment_recipe(
    fingerprint: &InsertedSegmentWorkflowFingerprint,
    segment_number: i32,
    prompt_language: PromptOutputLanguage,
) -> Result<VideoSegmentExecutionRecipeV1, AppError> {
    let constraints = &fingerprint.payload.constraints;
    let guidance_mode = constraints.guidance_mode;
    let workflow_mode = inserted_segment_workflow_mode(guidance_mode)?;
    let (reference_image_ids, guidance_reference_image_ids, continuity_source_video_id) =
        match guidance_mode {
            VideoSegmentGuidanceModeV1::Keyframes => {
                (Vec::new(), constraints.reference_image_ids.clone(), None)
            }
            VideoSegmentGuidanceModeV1::Continuity => (
                constraints.reference_image_ids.clone(),
                Vec::new(),
                constraints.continuity_source_video_id,
            ),
            VideoSegmentGuidanceModeV1::None | VideoSegmentGuidanceModeV1::SystemFrames => {
                (Vec::new(), Vec::new(), None)
            }
        };
    let recipe = VideoSegmentExecutionRecipeV1 {
        schema_version: VIDEO_SEGMENT_EXECUTION_RECIPE_V1.to_string(),
        segment_number,
        workflow_mode,
        style: fingerprint.payload.style.trim().to_string(),
        prompt_output_language: prompt_language.as_str().to_string(),
        duration_secs: constraints.selected_duration_secs,
        duration_secs_options: constraints.duration_secs_options.clone(),
        aspect_ratio: constraints.aspect_ratio.clone(),
        reference_image_ids,
        video_library_image_ids: Vec::new(),
        guidance_mode,
        guidance_reference_image_ids,
        continuity_source_video_id,
        reference_media: None,
        adopted_video_id: None,
    };
    recipe.validate().map_err(AppError::InvalidArgument)?;
    recipe
        .validate_generation_ready()
        .map_err(AppError::InvalidArgument)?;
    Ok(recipe)
}

pub(crate) fn inserted_segment_workflow_mode(
    guidance_mode: VideoSegmentGuidanceModeV1,
) -> Result<VideoSegmentWorkflowModeV1, AppError> {
    match guidance_mode {
        VideoSegmentGuidanceModeV1::Continuity => Ok(VideoSegmentWorkflowModeV1::FirstFrameChain),
        VideoSegmentGuidanceModeV1::Keyframes | VideoSegmentGuidanceModeV1::None => {
            Ok(VideoSegmentWorkflowModeV1::BoundaryFrames)
        }
        VideoSegmentGuidanceModeV1::SystemFrames => Err(AppError::InvalidArgument(
            "插入片段不支持 system_frames 引导".to_string(),
        )),
    }
}

async fn validate_inserted_continuity_source(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    recipe: &VideoSegmentExecutionRecipeV1,
) -> Result<(), AppError> {
    let Some(video_id) = recipe.continuity_source_video_id else {
        return Ok(());
    };
    let videos = service
        .generated_videos_for_chapter(project_id, chapter_id)
        .await?;
    let video = videos
        .iter()
        .find(|video| {
            video.id == video_id && video.status == generated_video::STATUS_COMPLETED
        })
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "插入片段续接来源视频 #{video_id} 已失效，请刷新时间线后重试"
            ))
        })?;
    if video.project_id != project_id || video.chapter_id != Some(chapter_id) {
        return Err(AppError::InvalidArgument(
            "插入片段续接来源视频不属于当前章节".to_string(),
        ));
    }
    let images = service
        .generated_images_for_video_prompt_references(project_id, chapter_id)
        .await?;
    if !images
        .iter()
        .any(|image| crate::video::tail_frame_image_belongs_to_video(image, video_id))
    {
        return Err(AppError::InvalidArgument(format!(
            "插入片段续接来源视频 #{video_id} 已没有可用尾帧，请刷新后重试"
        )));
    }
    Ok(())
}

pub(crate) fn timeline_revision_conflict(current: i64, expected: i64) -> AppError {
    AppError::InvalidState {
        current: format!("timeline_revision_conflict:current={current}"),
        operation: format!("expected={expected}"),
    }
}

fn native_timeline_needs_reconcile(
    snapshot: &a4n_db::video::VideoTimelineSnapshot,
    keys: &[NativeVideoTimelineKey],
) -> Result<bool, AppError> {
    let order =
        a4n_db::video::active_timeline_order(snapshot).map_err(|error| AppError::InvalidState {
            current: error.to_string(),
            operation: "检查章节视频时间线同步状态".to_string(),
        })?;
    let by_id = snapshot
        .items
        .iter()
        .map(|item| (item.id, item))
        .collect::<HashMap<_, _>>();
    let mut native = Vec::with_capacity(keys.len());
    for item_id in order {
        let Some(item) = by_id.get(&item_id) else {
            return Ok(true);
        };
        if item.item_kind != a4n_db::video::TIMELINE_NATIVE_KIND {
            continue;
        }
        let (Some(source_shot_id), Some(segment_index)) = (
            item.native_source_shot_id.as_deref(),
            item.native_segment_index_in_shot,
        ) else {
            return Ok(true);
        };
        if item.status != a4n_db::video::TIMELINE_ACTIVE_STATUS {
            return Ok(true);
        }
        native.push((source_shot_id, segment_index));
    }
    if native.len() != keys.len() {
        return Ok(true);
    }
    Ok(native.iter().zip(keys).any(|((shot, index), key)| {
        *shot != key.source_shot_id || *index != key.segment_index_in_shot
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::{
        ai::generated_video,
        chapter::{chapter_video_timeline, chapter_video_timeline_item},
    };
    use chrono::Utc;

    fn native_item(
        id: i64,
        source_shot_id: &str,
        position: i32,
    ) -> chapter_video_timeline_item::Model {
        let now = Utc::now().naive_utc();
        chapter_video_timeline_item::Model {
            id,
            timeline_id: 1,
            position: Some(position),
            item_kind: a4n_db::video::TIMELINE_NATIVE_KIND.to_string(),
            status: a4n_db::video::TIMELINE_ACTIVE_STATUS.to_string(),
            native_source_shot_id: Some(source_shot_id.to_string()),
            native_segment_index_in_shot: Some(1),
            insert_plan_prompt_id: None,
            workflow_run_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn inserted_item(id: i64) -> chapter_video_timeline_item::Model {
        let now = Utc::now().naive_utc();
        chapter_video_timeline_item::Model {
            id,
            timeline_id: 1,
            position: Some(1),
            item_kind: a4n_db::video::TIMELINE_INSERTED_KIND.to_string(),
            status: a4n_db::video::TIMELINE_ACTIVE_STATUS.to_string(),
            native_source_shot_id: None,
            native_segment_index_in_shot: None,
            insert_plan_prompt_id: Some(99),
            workflow_run_id: Some(1),
            created_at: now,
            updated_at: now,
        }
    }

    fn completed_inserted_video(id: i64, timeline_item_id: i64) -> generated_video::Model {
        let now = Utc::now().naive_utc();
        generated_video::Model {
            id,
            project_id: 1,
            chapter_id: Some(1),
            video_type: generated_video::VIDEO_TYPE_CHAPTER.to_string(),
            workflow_mode: Some("boundary_frames".to_string()),
            file_path: Some(format!("{id}.mp4")),
            thumbnail_path: None,
            prompt: "prompt".to_string(),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1920,
            height: 1080,
            duration_secs: 6.0,
            fps: 24,
            file_size: None,
            source_video_id: None,
            source_image_id: None,
            status: "completed".to_string(),
            provider_job_id: None,
            error_message: None,
            timeline_item_id: Some(timeline_item_id),
            metadata_json: None,
            is_favorite: false,
            created_at: now,
            updated_at: now,
        }
    }

    fn snapshot() -> a4n_db::video::VideoTimelineSnapshot {
        let now = Utc::now().naive_utc();
        a4n_db::video::VideoTimelineSnapshot {
            timeline: chapter_video_timeline::Model {
                id: 1,
                project_id: 1,
                chapter_id: 1,
                workflow_mode: chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN.to_string(),
                revision: 1,
                created_at: now,
                updated_at: now,
            },
            items: vec![native_item(1, "shot-a", 1), native_item(2, "shot-b", 2)],
        }
    }

    fn inserted_fingerprint(
        guidance_mode: VideoSegmentGuidanceModeV1,
    ) -> InsertedSegmentWorkflowFingerprint {
        InsertedSegmentWorkflowFingerprint {
            schema_version: 1,
            workflow_kind: INSERTED_SEGMENT_WORKFLOW_KIND.to_string(),
            project_id: 1,
            chapter_id: 1,
            source_content_sha256: "source".to_string(),
            context_sha256: "context".to_string(),
            timeline_id: 1,
            timeline_revision: 1,
            gap: VideoTimelineGap {
                previous_item_id: Some(1),
                next_item_id: Some(2),
            },
            payload: InsertedSegmentPayload {
                user_prompt: "反应镜头".to_string(),
                style: "电影感".to_string(),
                prompt_output_language: "zh_cn".to_string(),
                constraints: InsertedSegmentConstraints {
                    selected_duration_secs: 6,
                    duration_secs_options: vec![6],
                    aspect_ratio: "16:9".to_string(),
                    guidance_mode,
                    continuity_source_video_id: Some(9),
                    reference_image_ids: vec![21],
                },
            },
        }
    }

    #[test]
    fn inserted_recipe_routes_the_single_reference_source_by_guidance_mode() {
        let keyframes = inserted_segment_recipe(
            &inserted_fingerprint(VideoSegmentGuidanceModeV1::Keyframes),
            2,
            PromptOutputLanguage::ZhCn,
        )
        .unwrap();
        assert!(keyframes.reference_image_ids.is_empty());
        assert_eq!(keyframes.guidance_reference_image_ids, vec![21]);
        assert_eq!(keyframes.continuity_source_video_id, None);

        let continuity = inserted_segment_recipe(
            &inserted_fingerprint(VideoSegmentGuidanceModeV1::Continuity),
            2,
            PromptOutputLanguage::ZhCn,
        )
        .unwrap();
        assert_eq!(continuity.reference_image_ids, vec![21]);
        assert!(continuity.guidance_reference_image_ids.is_empty());
        assert_eq!(continuity.continuity_source_video_id, Some(9));
    }

    #[test]
    fn inserted_recipe_rejects_system_frame_guidance() {
        assert!(
            inserted_segment_recipe(
                &inserted_fingerprint(VideoSegmentGuidanceModeV1::SystemFrames),
                2,
                PromptOutputLanguage::ZhCn,
            )
            .is_err()
        );
    }

    #[test]
    fn native_reconcile_detects_order_changes_even_when_identity_fields_match() {
        let matching = vec![
            NativeVideoTimelineKey::new("shot-a", 1).unwrap(),
            NativeVideoTimelineKey::new("shot-b", 1).unwrap(),
        ];
        let reordered = vec![matching[1].clone(), matching[0].clone()];

        assert!(!native_timeline_needs_reconcile(&snapshot(), &matching).unwrap());
        assert!(native_timeline_needs_reconcile(&snapshot(), &reordered).unwrap());
    }

    #[test]
    fn assembly_order_excludes_items_that_need_reanchoring() {
        let mut snapshot = snapshot();
        snapshot.items[1].position = Some(3);
        let now = Utc::now().naive_utc();
        snapshot.items.push(chapter_video_timeline_item::Model {
            id: 9,
            timeline_id: 1,
            position: Some(2),
            item_kind: a4n_db::video::TIMELINE_INSERTED_KIND.to_string(),
            status: a4n_db::video::TIMELINE_NEEDS_REANCHOR_STATUS.to_string(),
            native_source_shot_id: None,
            native_segment_index_in_shot: None,
            insert_plan_prompt_id: Some(99),
            workflow_run_id: Some(9),
            created_at: now,
            updated_at: now,
        });

        let order = video_timeline_assembly_order_from_snapshot(&snapshot).unwrap();

        assert_eq!(order.display_index_by_item_id.get(&1), Some(&1));
        assert!(!order.display_index_by_item_id.contains_key(&9));
        assert_eq!(order.display_index_by_item_id.get(&2), Some(&3));
    }

    #[test]
    fn assembly_order_resolves_inserted_and_native_video_identity() {
        let mut snapshot = snapshot();
        snapshot.items[1].position = Some(3);
        let mut item = inserted_item(9);
        item.position = Some(2);
        snapshot.items.insert(1, item);
        let order = video_timeline_assembly_order_from_snapshot(&snapshot).unwrap();
        let inserted = completed_inserted_video(3, 9);
        let mut native = completed_inserted_video(4, 99);
        native.timeline_item_id = None;
        native.metadata_json = Some(json!({
            "video_execution_identity": {
                "source_shot_id": "shot-b",
                "segment_index_in_shot": 1,
            },
        }));

        assert_eq!(order.display_index_for_video(&inserted), Some(2));
        assert_eq!(order.display_index_for_video(&native), Some(3));
    }

    #[test]
    fn inserted_current_video_uses_the_timeline_item_foreign_key() {
        let item = inserted_item(8);
        let videos = [
            completed_inserted_video(3, 8),
            completed_inserted_video(2, 8),
        ];
        assert_eq!(
            current_timeline_item_video_id(&item, Some(2), &videos).unwrap(),
            Some(2)
        );
        assert_eq!(
            current_timeline_item_video_id(&item, None, &videos).unwrap(),
            None
        );

        let mut invalid = completed_inserted_video(4, 8);
        invalid.timeline_item_id = Some(9);
        assert!(current_timeline_item_video_id(&item, Some(4), &[invalid]).is_err());
    }

    #[test]
    fn native_current_video_uses_stable_source_identity() {
        let item = native_item(8, "shot-a", 1);
        let mut video = completed_inserted_video(3, 99);
        video.metadata_json = Some(json!({
            "video_execution_identity": {
                "source_shot_id": "shot-a",
                "segment_index_in_shot": 1,
            },
        }));
        assert!(current_timeline_item_video_id(&item, Some(3), &[video.clone()]).is_err());
        video.timeline_item_id = None;

        assert_eq!(
            current_timeline_item_video_id(&item, Some(3), &[video]).unwrap(),
            Some(3)
        );
    }

    #[test]
    fn reanchor_item_cannot_be_used_as_a_new_insert_anchor() {
        let mut item = inserted_item(8);
        item.status = a4n_db::video::TIMELINE_NEEDS_REANCHOR_STATUS.to_string();

        assert!(timeline_item_context(Some(8), &[item], &[], &[], &[], &[], &[]).is_err());
    }
}
