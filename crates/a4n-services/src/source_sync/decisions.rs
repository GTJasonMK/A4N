use super::*;
use std::collections::{BTreeSet, HashMap};

use crate::galgame::{
    GalgamePipelinePromptSourceSpan, GalgameSceneRegenerationInput, GalgameSceneRegenerationPlan,
    LINE_TYPE_CHOICE, SCENE_ENTRY_MAIN, normalize_line_type, normalize_position,
    validate_line_semantics, validate_presentation_json,
    validate_source_sync_scene_regeneration_stage_outputs,
};

#[derive(Debug, Clone, Default)]
pub(super) struct PreparedSourceSyncDecisionContext {
    galgame_scene_regeneration_plans: HashMap<i64, GalgameSceneRegenerationPlan>,
}

pub(super) async fn prepare_decision_execution_context<C>(
    db: &C,
    run: &source_sync_run::Model,
    decisions: &[SourceSyncDecisionInput],
    execution_context: SourceSyncExecutionContext<'_>,
) -> Result<PreparedSourceSyncDecisionContext, AppError>
where
    C: ConnectionTrait,
{
    let mut prepared = PreparedSourceSyncDecisionContext::default();
    let Some(galgame_execution) = execution_context.galgame else {
        return Ok(prepared);
    };
    for decision in decisions {
        if normalize_strategy(&decision.strategy)? != "regenerate_scene" {
            continue;
        }
        let conflict = source_sync_conflict::Entity::find_by_id(decision.conflict_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::NotFound(format!("源同步冲突不存在: {}", decision.conflict_id))
            })?;
        if conflict.sync_run_id != run.id {
            return Err(AppError::InvalidArgument(format!(
                "冲突 {} 不属于同步运行 {}",
                conflict.id, run.id
            )));
        }
        if conflict.status != CONFLICT_STATUS_PENDING {
            return Err(AppError::InvalidState {
                current: conflict.status,
                operation: "prepare_source_sync_decision".to_string(),
            });
        }
        let payload: RegenerateGalgameScenePayload =
            parse_decision_payload(&decision.decision_payload_json)?;
        let plan = build_prepared_galgame_scene_regeneration_plan(
            db,
            run,
            &conflict,
            &payload,
            galgame_execution,
        )
        .await?;
        prepared
            .galgame_scene_regeneration_plans
            .insert(conflict.id, plan);
    }
    Ok(prepared)
}

pub(super) async fn apply_decision<C>(
    db: &C,
    run: &source_sync_run::Model,
    decision: SourceSyncDecisionInput,
    prepared_context: &PreparedSourceSyncDecisionContext,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let strategy = normalize_strategy(&decision.strategy)?;
    let conflict = source_sync_conflict::Entity::find_by_id(decision.conflict_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("源同步冲突不存在: {}", decision.conflict_id)))?;
    if conflict.sync_run_id != run.id {
        return Err(AppError::InvalidArgument(format!(
            "冲突 {} 不属于同步运行 {}",
            conflict.id, run.id
        )));
    }
    if conflict.status != CONFLICT_STATUS_PENDING {
        return Err(AppError::InvalidState {
            current: conflict.status,
            operation: "apply_source_sync_decision".to_string(),
        });
    }
    let mut decision_payload_json = decision.decision_payload_json.clone();
    match strategy {
        "keep_local" => {
            resolve_conflict(
                db,
                conflict,
                strategy,
                CONFLICT_STATUS_IGNORED,
                decision_payload_json.clone(),
            )
            .await?;
        }
        "detach" => {
            novel_project::Entity::update_many()
                .col_expr(
                    novel_project::Column::SourceSyncStatus,
                    Expr::value(SOURCE_SYNC_DETACHED),
                )
                .filter(novel_project::Column::Id.eq(run.derived_project_id))
                .exec(db)
                .await
                .map_err(AppError::Database)?;
            resolve_conflict(
                db,
                conflict,
                strategy,
                CONFLICT_STATUS_IGNORED,
                decision_payload_json.clone(),
            )
            .await?;
        }
        "rebind_anchor" => {
            ensure_clean_sync_target(&conflict, strategy)?;
            let payload: RebindAnchorPayload = parse_decision_payload(&decision_payload_json)?;
            ensure_decision_targets_generation_spec(
                &conflict,
                payload.generation_spec_id,
                strategy,
            )?;
            if payload.anchor_mappings.is_empty() {
                return Err(AppError::InvalidArgument(
                    "decision_payload_json.anchor_mappings 不能为空".to_string(),
                ));
            }
            let spec = load_run_generation_spec(db, run, payload.generation_spec_id).await?;
            for mapping in &payload.anchor_mappings {
                validate_new_anchor(
                    db,
                    run.root_project_id,
                    run.to_snapshot_id,
                    mapping.new_text_anchor_id,
                )
                .await?;
                let result = generation_spec_text_anchor::Entity::update_many()
                    .col_expr(
                        generation_spec_text_anchor::Column::TextAnchorId,
                        Expr::value(mapping.new_text_anchor_id),
                    )
                    .filter(
                        generation_spec_text_anchor::Column::GenerationSpecId
                            .eq(payload.generation_spec_id),
                    )
                    .filter(
                        generation_spec_text_anchor::Column::TextAnchorId
                            .eq(mapping.old_text_anchor_id),
                    )
                    .exec(db)
                    .await
                    .map_err(AppError::Database)?;
                if result.rows_affected == 0 {
                    return Err(AppError::InvalidArgument(format!(
                        "生成意图 {} 未引用旧文本锚点 {}",
                        payload.generation_spec_id, mapping.old_text_anchor_id
                    )));
                }
            }
            ensure_spec_anchors_belong_to_snapshot(
                db,
                payload.generation_spec_id,
                run.to_snapshot_id,
            )
            .await?;
            update_spec_to_snapshot(db, &spec, run.to_snapshot_id).await?;
            resolve_conflict(
                db,
                conflict,
                strategy,
                CONFLICT_STATUS_RESOLVED,
                decision_payload_json.clone(),
            )
            .await?;
        }
        "rebuild_prompt" | "adopt_source" => {
            ensure_clean_sync_target(&conflict, strategy)?;
            let payload: RefreshSpecPayload = parse_decision_payload(&decision_payload_json)?;
            ensure_decision_targets_generation_spec(
                &conflict,
                payload.generation_spec_id,
                strategy,
            )?;
            let updated_spec = refresh_spec_for_snapshot(db, run, payload).await?;
            render_and_insert_prompt_artifact(db, &updated_spec, strategy).await?;
            resolve_conflict(
                db,
                conflict,
                strategy,
                CONFLICT_STATUS_RESOLVED,
                decision_payload_json.clone(),
            )
            .await?;
        }
        "regenerate_scene" => {
            let payload: RegenerateGalgameScenePayload =
                parse_decision_payload(&decision_payload_json)?;
            let plan = require_galgame_scene_regeneration_plan(prepared_context, conflict.id)?;
            let result =
                regenerate_galgame_scene_from_snapshot(db, run, &conflict, &payload, plan).await?;
            decision_payload_json = json!({
                "user_action": "regenerate_scene",
                "galgame_scene_id": payload.galgame_scene_id,
                "reason": payload.reason,
                "result": result,
            });
            resolve_conflict(
                db,
                conflict,
                strategy,
                CONFLICT_STATUS_RESOLVED,
                decision_payload_json.clone(),
            )
            .await?;
        }
        "manual" => {
            let payload: ManualPromptPayload = parse_decision_payload(&decision_payload_json)?;
            let artifact = prompt_artifact::Entity::find_by_id(payload.prompt_artifact_id)
                .one(db)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| {
                    AppError::NotFound(format!("提示词产物不存在: {}", payload.prompt_artifact_id))
                })?;
            ensure_decision_targets_generation_spec(
                &conflict,
                artifact.generation_spec_id,
                strategy,
            )?;
            let spec = load_run_generation_spec(db, run, artifact.generation_spec_id).await?;
            insert_prompt_artifact(
                db,
                spec.id,
                &artifact.prompt_kind,
                &payload.prompt_text,
                "source-sync-manual-v1",
                true,
            )
            .await?;
            resolve_conflict(
                db,
                conflict,
                strategy,
                CONFLICT_STATUS_RESOLVED,
                decision_payload_json.clone(),
            )
            .await?;
        }
        "create_replacement" => {
            let payload: ReplacementSpecPayload = parse_decision_payload(&decision_payload_json)?;
            ensure_decision_targets_generation_spec(
                &conflict,
                payload.generation_spec_id,
                strategy,
            )?;
            let new_spec = create_replacement_spec(db, run, payload).await?;
            render_and_insert_prompt_artifact(db, &new_spec, strategy).await?;
            resolve_conflict(
                db,
                conflict,
                strategy,
                CONFLICT_STATUS_RESOLVED,
                decision_payload_json.clone(),
            )
            .await?;
        }
        "delete_local" => {
            if conflict.local_state_kind != "clean" && conflict.local_state_kind != "draft" {
                return Err(AppError::InvalidArgument(
                    "delete_local 只能处理 clean 或 draft 本地对象".to_string(),
                ));
            }
            let generation_spec_id = generation_spec_id_for_conflict(&conflict, strategy)?;
            delete_local_generation_spec(db, run, generation_spec_id).await?;
            resolve_conflict(
                db,
                conflict,
                strategy,
                CONFLICT_STATUS_RESOLVED,
                decision_payload_json.clone(),
            )
            .await?;
        }
        _ => unreachable!(),
    }
    source_sync_decision::ActiveModel {
        conflict_id: Set(decision.conflict_id),
        decided_by: Set("user".to_string()),
        strategy: Set(strategy.to_string()),
        decision_payload_json: Set(decision_payload_json),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub(super) fn require_galgame_scene_regeneration_plan(
    prepared_context: &PreparedSourceSyncDecisionContext,
    conflict_id: i64,
) -> Result<&GalgameSceneRegenerationPlan, AppError> {
    prepared_context
        .galgame_scene_regeneration_plans
        .get(&conflict_id)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("missing_galgame_scene_regeneration_plan_for_conflict_{conflict_id}"),
            operation: "regenerate_scene".to_string(),
        })
}

struct GalgameSceneRegenerationSource {
    scene: galgame_scene::Model,
    script: galgame_script::Model,
    old_snapshot_chapter: source_snapshot_chapter::Model,
    new_snapshot_chapter: source_snapshot_chapter::Model,
    anchors: Vec<text_anchor::Model>,
}

async fn build_prepared_galgame_scene_regeneration_plan<C>(
    db: &C,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    payload: &RegenerateGalgameScenePayload,
    galgame_execution: GalgameSourceSyncExecutionContext<'_>,
) -> Result<GalgameSceneRegenerationPlan, AppError>
where
    C: ConnectionTrait,
{
    let source = load_galgame_scene_regeneration_source(db, run, conflict, payload).await?;
    let speaker_profiles = galgame_execution
        .service
        .speaker_profiles_for_root_project(run.root_project_id)
        .await?;
    galgame_execution
        .service
        .build_source_sync_scene_regeneration(
            GalgameSceneRegenerationInput {
                project_id: run.derived_project_id,
                root_project_id: run.root_project_id,
                workflow_run_id: galgame_execution.workflow_run_id,
                source_snapshot_id: run.to_snapshot_id,
                source_snapshot_chapter_id: source.new_snapshot_chapter.id,
                source_chapter_id: source.new_snapshot_chapter.source_chapter_id,
                chapter_number: source.new_snapshot_chapter.chapter_number,
                chapter_title: source.new_snapshot_chapter.title.clone(),
                chapter_summary: source.new_snapshot_chapter.summary.clone(),
                chapter_content_hash: source.new_snapshot_chapter.content_hash.clone(),
                chapter_content: source.new_snapshot_chapter.content.clone(),
                scene_id: source.scene.id,
                scene_index: source.scene.scene_index,
                scene_title: source.scene.title.clone(),
                location: source.scene.location.clone(),
                time_of_day: source.scene.time_of_day.clone(),
                background_note: source.scene.background_note.clone(),
                source_spans: galgame_regeneration_source_spans(
                    &source.new_snapshot_chapter,
                    &source.anchors,
                ),
                speaker_profiles,
            },
            galgame_execution.llm,
            galgame_execution.prompt_engine,
        )
        .await
}

async fn load_galgame_scene_regeneration_source<C>(
    db: &C,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    payload: &RegenerateGalgameScenePayload,
) -> Result<GalgameSceneRegenerationSource, AppError>
where
    C: ConnectionTrait,
{
    if conflict.conflict_scope != "text_anchor"
        || !matches!(
            conflict.target_kind.as_str(),
            "galgame_scene" | "galgame_line"
        )
    {
        return Err(AppError::InvalidArgument(format!(
            "regenerate_scene 只支持 Galgame 场/行文本锚点冲突，当前为 {}/{}",
            conflict.conflict_scope, conflict.target_kind
        )));
    }

    let scene_id = match conflict.target_kind.as_str() {
        "galgame_scene" => conflict.target_id,
        "galgame_line" => {
            let line = galgame_line::Entity::find_by_id(conflict.target_id)
                .one(db)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Galgame 行不存在: {}", conflict.target_id))
                })?;
            line.scene_id
        }
        _ => unreachable!(),
    };
    if payload.galgame_scene_id != scene_id {
        return Err(AppError::InvalidArgument(format!(
            "regenerate_scene payload 指向 Galgame 场 {}，但冲突目标属于场 {}",
            payload.galgame_scene_id, scene_id
        )));
    }

    let scene = galgame_scene::Entity::find_by_id(scene_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Galgame 场不存在: {scene_id}")))?;
    if scene.entry_kind != "main" {
        return Err(AppError::InvalidArgument(format!(
            "regenerate_scene 当前只支持 main 场，Galgame 场 {} 是 {}",
            scene.id, scene.entry_kind
        )));
    }
    let script = galgame_script::Entity::find_by_id(scene.script_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Galgame 剧本不存在: {}", scene.script_id)))?;
    if script.project_id != run.derived_project_id || script.root_project_id != run.root_project_id
    {
        return Err(AppError::InvalidArgument(format!(
            "Galgame 场 {} 不属于本次源同步运行",
            scene.id
        )));
    }

    let old_snapshot_chapter =
        source_snapshot_chapter::Entity::find_by_id(scene.source_snapshot_chapter_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "旧源快照章节不存在: {}",
                    scene.source_snapshot_chapter_id
                ))
            })?;
    if old_snapshot_chapter.snapshot_id != run.from_snapshot_id {
        return Err(AppError::InvalidArgument(format!(
            "Galgame 场 {} 的源快照章节不属于本次同步起点",
            scene.id
        )));
    }
    let new_snapshot_chapter = source_snapshot_chapter_for_source(
        db,
        run.to_snapshot_id,
        old_snapshot_chapter.source_chapter_id,
    )
    .await?;
    let anchors = text_anchor::Entity::find()
        .filter(text_anchor::Column::ProjectId.eq(run.root_project_id))
        .filter(text_anchor::Column::SourceSnapshotId.eq(run.to_snapshot_id))
        .filter(text_anchor::Column::SourceSnapshotChapterId.eq(new_snapshot_chapter.id))
        .order_by_asc(text_anchor::Column::ParagraphIndex)
        .order_by_asc(text_anchor::Column::StartOffset)
        .order_by_asc(text_anchor::Column::EndOffset)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if anchors.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "目标源快照章节 {} 没有可用于重建 Galgame 场的文本锚点",
            new_snapshot_chapter.id
        )));
    }

    Ok(GalgameSceneRegenerationSource {
        scene,
        script,
        old_snapshot_chapter,
        new_snapshot_chapter,
        anchors,
    })
}

fn galgame_regeneration_source_spans(
    chapter: &source_snapshot_chapter::Model,
    anchors: &[text_anchor::Model],
) -> Vec<GalgamePipelinePromptSourceSpan> {
    anchors
        .iter()
        .map(|anchor| GalgamePipelinePromptSourceSpan {
            id: format!("ch{}:ta{}", chapter.chapter_number, anchor.id),
            text_anchor_id: anchor.id,
            source_snapshot_chapter_id: chapter.id,
            paragraph_index: anchor.paragraph_index,
            start_offset: anchor.start_offset,
            end_offset: anchor.end_offset,
            text: anchor.quote.clone(),
        })
        .collect()
}

async fn regenerate_galgame_scene_from_snapshot<C>(
    db: &C,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    payload: &RegenerateGalgameScenePayload,
    llm_plan: &GalgameSceneRegenerationPlan,
) -> Result<Value, AppError>
where
    C: ConnectionTrait,
{
    let source = load_galgame_scene_regeneration_source(db, run, conflict, payload).await?;
    let scene = source.scene;
    let script = source.script;
    let old_snapshot_chapter = source.old_snapshot_chapter;
    let new_snapshot_chapter = source.new_snapshot_chapter;

    let new_anchor_ids = source
        .anchors
        .iter()
        .map(|anchor| anchor.id)
        .collect::<BTreeSet<_>>();
    validate_galgame_scene_regeneration_plan_for_apply(
        llm_plan,
        scene.scene_index,
        new_snapshot_chapter.id,
        &new_anchor_ids,
    )?;

    let old_lines = galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.eq(scene.id))
        .order_by_asc(galgame_line::Column::LineIndex)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let old_line_ids = old_lines.iter().map(|line| line.id).collect::<Vec<_>>();
    let mut generation_spec_ids =
        galgame_generation_specs_for_scene(db, run.derived_project_id, scene.id, &old_line_ids)
            .await?;
    let portrait_generation_spec_ids = galgame_portrait_generation_specs_for_source_lines(
        db,
        run.derived_project_id,
        &old_line_ids,
    )
    .await?;
    let deleted_portrait_generation_spec_count = portrait_generation_spec_ids.len();
    generation_spec_ids.extend(portrait_generation_spec_ids);
    generation_spec_ids.sort_unstable();
    generation_spec_ids.dedup();
    let mut binding_condition = Condition::any().add(
        Condition::all()
            .add(media_binding::Column::TargetKind.eq("galgame_scene"))
            .add(media_binding::Column::TargetId.eq(scene.id)),
    );
    if !old_line_ids.is_empty() {
        binding_condition = binding_condition.add(
            Condition::all()
                .add(media_binding::Column::TargetKind.eq("galgame_line"))
                .add(media_binding::Column::TargetId.is_in(old_line_ids.clone())),
        );
    }
    let bound_media_asset_ids = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(binding_condition.clone())
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let provenance_media_asset_ids =
        media_asset_ids_for_generation_specs(db, &generation_spec_ids).await?;

    if !old_line_ids.is_empty() {
        galgame_choice::Entity::delete_many()
            .filter(galgame_choice::Column::LineId.is_in(old_line_ids.clone()))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }
    media_binding::Entity::delete_many()
        .filter(binding_condition)
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    delete_unbound_project_media_assets(
        db,
        run.derived_project_id,
        bound_media_asset_ids
            .into_iter()
            .chain(provenance_media_asset_ids)
            .collect(),
    )
    .await?;
    delete_generation_specs(db, &generation_spec_ids).await?;
    if !old_line_ids.is_empty() {
        galgame_line::Entity::delete_many()
            .filter(galgame_line::Column::Id.is_in(old_line_ids.clone()))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }

    let now = Utc::now().naive_utc();
    let generated_scene = llm_plan.scene.clone();
    for line in &generated_scene.lines {
        galgame_line::ActiveModel {
            scene_id: Set(scene.id),
            line_index: Set(line.line_index),
            line_type: Set(line.line_type.clone()),
            speaker_profile_id: Set(line.speaker_profile_id),
            text: Set(line.text.clone()),
            expression: Set(line.expression.clone()),
            pose: Set(line.pose.clone()),
            position: Set(line.position.clone()),
            presentation_json: Set(line.presentation_json.clone()),
            text_anchor_id: Set(line.text_anchor_id),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)?;
    }
    let mut active_scene: galgame_scene::ActiveModel = scene.clone().into();
    active_scene.source_snapshot_chapter_id = Set(new_snapshot_chapter.id);
    active_scene.title = Set(generated_scene.title);
    active_scene.location = Set(generated_scene.location);
    active_scene.time_of_day = Set(generated_scene.time_of_day);
    active_scene.participants_json = Set(generated_scene.participants_json);
    active_scene.background_note = Set(generated_scene.background_note);
    active_scene.updated_at = Set(now);
    active_scene.update(db).await.map_err(AppError::Database)?;

    let mut active_script: galgame_script::ActiveModel = script.into();
    active_script.source_snapshot_id = Set(run.to_snapshot_id);
    active_script.updated_at = Set(now);
    active_script.update(db).await.map_err(AppError::Database)?;

    let related_resolved =
        resolve_related_galgame_scene_conflicts(db, run, conflict.id, scene.id, &old_line_ids)
            .await?;

    Ok(json!({
        "galgame_scene_id": scene.id,
        "old_source_snapshot_chapter_id": old_snapshot_chapter.id,
        "new_source_snapshot_chapter_id": new_snapshot_chapter.id,
        "old_line_count": old_line_ids.len(),
        "new_line_count": generated_scene.lines.len(),
        "deleted_generation_spec_count": generation_spec_ids.len(),
        "deleted_portrait_generation_spec_count": deleted_portrait_generation_spec_count,
        "resolved_related_conflict_count": related_resolved,
        "mode": "llm_partial_regeneration",
        "stage_outputs": llm_plan.stage_outputs_json.clone(),
    }))
}

pub(super) fn validate_galgame_scene_regeneration_plan_for_apply(
    llm_plan: &GalgameSceneRegenerationPlan,
    expected_scene_index: i32,
    expected_source_snapshot_chapter_id: i64,
    valid_text_anchor_ids: &BTreeSet<i64>,
) -> Result<(), AppError> {
    validate_source_sync_scene_regeneration_stage_outputs(&llm_plan.stage_outputs_json)?;
    let scene = &llm_plan.scene;
    if scene.scene_index != expected_scene_index {
        return Err(AppError::InvalidArgument(format!(
            "Galgame source sync 重建计划 scene_index 不匹配: expected={}, actual={}",
            expected_scene_index, scene.scene_index
        )));
    }
    if scene.source_snapshot_chapter_id != expected_source_snapshot_chapter_id {
        return Err(AppError::InvalidArgument(format!(
            "Galgame source sync 重建计划 source_snapshot_chapter_id 不匹配: expected={}, actual={}",
            expected_source_snapshot_chapter_id, scene.source_snapshot_chapter_id
        )));
    }
    if scene.entry_kind != SCENE_ENTRY_MAIN {
        return Err(AppError::InvalidArgument(format!(
            "Galgame source sync 重建计划只能写回 main 场，当前为 {}",
            scene.entry_kind
        )));
    }
    if scene.next_scene_index.is_some() {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 重建计划不能改写场跳转关系".to_string(),
        ));
    }
    if scene.title.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 重建计划场标题不能为空".to_string(),
        ));
    }
    if scene
        .background_note
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 重建计划必须保留 background_note".to_string(),
        ));
    }
    if !scene.participants_json.is_array() {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 重建计划 participants_json 必须是 JSON 数组".to_string(),
        ));
    }
    if scene.lines.is_empty() {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 重建计划不能写入空行集合".to_string(),
        ));
    }
    for (index, line) in scene.lines.iter().enumerate() {
        let expected_line_index = i32::try_from(index + 1).map_err(|_| {
            AppError::InvalidArgument(
                "Galgame source sync 重建计划 line_index 超出 i32 范围".to_string(),
            )
        })?;
        if line.line_index != expected_line_index {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划 line_index 必须连续: expected={}, actual={}",
                expected_line_index, line.line_index
            )));
        }
        let normalized_line_type = normalize_line_type(&line.line_type)?;
        if normalized_line_type != line.line_type {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划 line_type 未规范化: {}",
                line.line_type
            )));
        }
        if normalized_line_type == LINE_TYPE_CHOICE || !line.choices.is_empty() {
            return Err(AppError::InvalidArgument(
                "Galgame source sync 场级重建不允许写入选择支；请重新运行完整 Galgame 管线"
                    .to_string(),
            ));
        }
        if line.text.trim().is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行文本不能为空",
                line.line_index
            )));
        }
        if let Some(speaker_profile_id) = line.speaker_profile_id
            && speaker_profile_id <= 0
        {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行 speaker_profile_id 必须是正整数",
                line.line_index
            )));
        }
        if let Some(expression) = line.expression.as_deref()
            && expression.trim() != expression
        {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行 expression 未规范化",
                line.line_index
            )));
        }
        validate_line_semantics(
            normalized_line_type,
            line.speaker_profile_id,
            line.expression.as_deref(),
        )?;
        if normalize_position(line.position.clone())? != line.position {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行 position 未规范化",
                line.line_index
            )));
        }
        validate_presentation_json(
            &line.presentation_json,
            &format!("Galgame source sync 重建计划第 {} 行", line.line_index),
        )?;
        let Some(text_anchor_id) = line.text_anchor_id else {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行缺少 text_anchor_id",
                line.line_index
            )));
        };
        if !valid_text_anchor_ids.contains(&text_anchor_id) {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行 text_anchor_id 不属于目标源快照章节: {}",
                line.line_index, text_anchor_id
            )));
        }
    }

    Ok(())
}

async fn galgame_generation_specs_for_scene<C>(
    db: &C,
    project_id: i64,
    scene_id: i64,
    line_ids: &[i64],
) -> Result<Vec<i64>, AppError>
where
    C: ConnectionTrait,
{
    let mut condition = Condition::any().add(
        Condition::all()
            .add(generation_spec::Column::TargetEntityKind.eq("galgame_scene"))
            .add(generation_spec::Column::TargetEntityId.eq(scene_id)),
    );
    if !line_ids.is_empty() {
        condition = condition.add(
            Condition::all()
                .add(generation_spec::Column::TargetEntityKind.eq("galgame_line"))
                .add(generation_spec::Column::TargetEntityId.is_in(line_ids.to_vec())),
        );
    }
    generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(condition)
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)
}

async fn galgame_portrait_generation_specs_for_source_lines<C>(
    db: &C,
    project_id: i64,
    line_ids: &[i64],
) -> Result<Vec<i64>, AppError>
where
    C: ConnectionTrait,
{
    if line_ids.is_empty() {
        return Ok(Vec::new());
    }
    let line_ids = line_ids.iter().copied().collect::<BTreeSet<_>>();
    let rows = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetKind.eq("portrait_expression"))
        .filter(generation_spec::Column::TargetEntityKind.eq("character_profile"))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(rows
        .into_iter()
        .filter(|spec| galgame_portrait_spec_references_lines(&spec.params_json, &line_ids))
        .map(|spec| spec.id)
        .collect())
}

pub(super) fn galgame_portrait_spec_references_lines(
    params_json: &Value,
    line_ids: &BTreeSet<i64>,
) -> bool {
    params_json
        .get("source_line_ids")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .filter_map(Value::as_i64)
                .any(|line_id| line_ids.contains(&line_id))
        })
}

async fn media_asset_ids_for_generation_specs<C>(
    db: &C,
    generation_spec_ids: &[i64],
) -> Result<Vec<i64>, AppError>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(Vec::new());
    }
    media_provenance::Entity::find()
        .select_only()
        .column(media_provenance::Column::MediaAssetId)
        .filter(media_provenance::Column::GenerationSpecId.is_in(generation_spec_ids.to_vec()))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)
}

async fn delete_unbound_project_media_assets<C>(
    db: &C,
    project_id: i64,
    candidate_media_asset_ids: BTreeSet<i64>,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    if candidate_media_asset_ids.is_empty() {
        return Ok(());
    }
    let candidates = candidate_media_asset_ids
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let still_bound = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(media_binding::Column::MediaAssetId.is_in(candidates))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let delete_media_asset_ids = candidate_media_asset_ids
        .difference(&still_bound)
        .copied()
        .collect::<Vec<_>>();
    if delete_media_asset_ids.is_empty() {
        return Ok(());
    }
    let delete_media_asset_ids = media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::Id)
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .filter(media_asset::Column::Id.is_in(delete_media_asset_ids))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if delete_media_asset_ids.is_empty() {
        return Ok(());
    }
    media_provenance::Entity::delete_many()
        .filter(media_provenance::Column::MediaAssetId.is_in(delete_media_asset_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::MediaAssetId.is_in(delete_media_asset_ids.clone()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    media_asset::Entity::delete_many()
        .filter(media_asset::Column::Id.is_in(delete_media_asset_ids))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

async fn delete_generation_specs<C>(db: &C, generation_spec_ids: &[i64]) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(());
    }
    let prompt_artifact_ids = prompt_artifact::Entity::find()
        .select_only()
        .column(prompt_artifact::Column::Id)
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.to_vec()))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut provenance_update = media_provenance::Entity::update_many();
    provenance_update = provenance_update
        .col_expr(
            media_provenance::Column::GenerationSpecId,
            Expr::value(Option::<i64>::None),
        )
        .filter(media_provenance::Column::GenerationSpecId.is_in(generation_spec_ids.to_vec()));
    if !prompt_artifact_ids.is_empty() {
        provenance_update = provenance_update.col_expr(
            media_provenance::Column::PromptArtifactId,
            Expr::value(Option::<i64>::None),
        );
    }
    provenance_update
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    if !prompt_artifact_ids.is_empty() {
        media_provenance::Entity::update_many()
            .col_expr(
                media_provenance::Column::PromptArtifactId,
                Expr::value(Option::<i64>::None),
            )
            .filter(media_provenance::Column::PromptArtifactId.is_in(prompt_artifact_ids))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::GenerationSpecId.is_in(generation_spec_ids.to_vec()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_text_anchor::Entity::delete_many()
        .filter(
            generation_spec_text_anchor::Column::GenerationSpecId
                .is_in(generation_spec_ids.to_vec()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_entity_ref::Entity::delete_many()
        .filter(
            generation_spec_entity_ref::Column::GenerationSpecId
                .is_in(generation_spec_ids.to_vec()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    prompt_artifact::Entity::delete_many()
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.to_vec()))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec::Entity::delete_many()
        .filter(generation_spec::Column::Id.is_in(generation_spec_ids.to_vec()))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

async fn resolve_related_galgame_scene_conflicts<C>(
    db: &C,
    run: &source_sync_run::Model,
    current_conflict_id: i64,
    scene_id: i64,
    old_line_ids: &[i64],
) -> Result<u64, AppError>
where
    C: ConnectionTrait,
{
    let related = source_sync_conflict::Entity::find()
        .filter(source_sync_conflict::Column::SyncRunId.eq(run.id))
        .filter(source_sync_conflict::Column::Status.eq(CONFLICT_STATUS_PENDING))
        .filter(source_sync_conflict::Column::Id.ne(current_conflict_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let old_line_ids = old_line_ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let now = Utc::now().naive_utc();
    let mut resolved = 0u64;
    for conflict in related {
        if !is_same_galgame_scene_conflict(&conflict, scene_id, &old_line_ids) {
            continue;
        }
        let mut active: source_sync_conflict::ActiveModel = conflict.into();
        active.status = Set(CONFLICT_STATUS_RESOLVED.to_string());
        active.resolution_strategy = Set(Some("regenerate_scene".to_string()));
        active.decision_json = Set(Some(json!({
            "system_action": "resolved_by_regenerate_scene",
            "galgame_scene_id": scene_id,
        })));
        active.resolved_at = Set(Some(now));
        active.update(db).await.map_err(AppError::Database)?;
        resolved += 1;
    }
    Ok(resolved)
}

fn is_same_galgame_scene_conflict(
    conflict: &source_sync_conflict::Model,
    scene_id: i64,
    old_line_ids: &std::collections::BTreeSet<i64>,
) -> bool {
    if conflict.conflict_scope != "text_anchor" {
        return false;
    }
    match conflict.target_kind.as_str() {
        "galgame_scene" => conflict.target_id == scene_id,
        "galgame_line" => {
            conflict
                .affected_refs_json
                .get("galgame_scene_id")
                .and_then(Value::as_i64)
                == Some(scene_id)
                || old_line_ids.contains(&conflict.target_id)
        }
        _ => false,
    }
}

pub(super) fn ensure_decision_targets_generation_spec(
    conflict: &source_sync_conflict::Model,
    generation_spec_id: i64,
    strategy: &str,
) -> Result<(), AppError> {
    let expected = generation_spec_id_for_conflict(conflict, strategy)?;
    if generation_spec_id == expected {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{strategy} 决策目标生成意图不匹配：冲突 {} 影响 generation_spec={}，payload 指向 generation_spec={}",
            conflict.id, expected, generation_spec_id
        )))
    }
}

pub(super) fn generation_spec_id_for_conflict(
    conflict: &source_sync_conflict::Model,
    strategy: &str,
) -> Result<i64, AppError> {
    if conflict.conflict_scope == "generation_spec" {
        return Ok(conflict.target_id);
    }
    if conflict.conflict_scope == "text_anchor" && conflict.target_kind == "segment" {
        return Ok(conflict.target_id);
    }
    if let Some(generation_spec_id) = conflict
        .local_summary_json
        .get("generation_spec_id")
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
    {
        return Ok(generation_spec_id);
    }
    Err(AppError::InvalidArgument(format!(
        "{strategy} 决策要求冲突绑定生成意图，但冲突 {} 未提供 generation_spec_id",
        conflict.id
    )))
}

pub(super) async fn apply_unconflicted_items<C>(
    db: &C,
    run: &source_sync_run::Model,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let conflicts = source_sync_conflict::Entity::find()
        .filter(source_sync_conflict::Column::SyncRunId.eq(run.id))
        .filter(source_sync_conflict::Column::Status.eq(CONFLICT_STATUS_PENDING))
        .filter(source_sync_conflict::Column::Severity.eq("info"))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    for conflict in conflicts {
        let decision_payload = json!({
            "system_action": "apply_unconflicted_item",
            "source_change_kind": conflict.source_change_kind,
            "local_state_kind": conflict.local_state_kind,
        });
        let conflict_id = conflict.id;
        resolve_conflict(
            db,
            conflict,
            "keep_local",
            CONFLICT_STATUS_RESOLVED,
            decision_payload.clone(),
        )
        .await?;
        source_sync_decision::ActiveModel {
            conflict_id: Set(conflict_id),
            decided_by: Set("system".to_string()),
            strategy: Set("keep_local".to_string()),
            decision_payload_json: Set(decision_payload),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)?;
    }
    Ok(())
}

async fn resolve_conflict<C>(
    db: &C,
    conflict: source_sync_conflict::Model,
    strategy: &str,
    status: &str,
    decision_json: Value,
) -> Result<source_sync_conflict::Model, AppError>
where
    C: ConnectionTrait,
{
    let mut active: source_sync_conflict::ActiveModel = conflict.into();
    active.status = Set(status.to_string());
    active.resolution_strategy = Set(Some(strategy.to_string()));
    active.decision_json = Set(Some(decision_json));
    active.resolved_at = Set(Some(Utc::now().naive_utc()));
    active.update(db).await.map_err(AppError::Database)
}

async fn validate_new_anchor<C>(
    db: &C,
    root_project_id: i64,
    to_snapshot_id: i64,
    new_anchor_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let anchor = text_anchor::Entity::find_by_id(new_anchor_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("文本锚点不存在: {new_anchor_id}")))?;
    if anchor.project_id != root_project_id || anchor.source_snapshot_id != Some(to_snapshot_id) {
        return Err(AppError::InvalidArgument(format!(
            "新文本锚点 {} 不属于目标根项目和目标快照",
            new_anchor_id
        )));
    }
    Ok(())
}

async fn update_spec_to_snapshot<C>(
    db: &C,
    spec: &generation_spec::Model,
    to_snapshot_id: i64,
) -> Result<generation_spec::Model, AppError>
where
    C: ConnectionTrait,
{
    let snapshot = project_source_snapshot::Entity::find_by_id(to_snapshot_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("目标源快照不存在: {to_snapshot_id}")))?;
    let source_snapshot_chapter_id = match spec.source_chapter_id {
        Some(source_chapter_id) => Some(
            source_snapshot_chapter_for_source(db, to_snapshot_id, source_chapter_id)
                .await?
                .id,
        ),
        None => None,
    };
    let target_entity_id = if spec.target_entity_kind == "chapter" {
        source_snapshot_chapter_id
    } else {
        spec.target_entity_id
    };
    let mut active: generation_spec::ActiveModel = spec.clone().into();
    active.source_context_kind = Set(Some("snapshot_backed".to_string()));
    active.source_snapshot_id = Set(Some(snapshot.id));
    active.source_version_hash = Set(snapshot.source_version_hash);
    active.source_snapshot_chapter_id = Set(source_snapshot_chapter_id);
    active.target_entity_id = Set(target_entity_id);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
}

async fn refresh_spec_for_snapshot<C>(
    db: &C,
    run: &source_sync_run::Model,
    payload: RefreshSpecPayload,
) -> Result<generation_spec::Model, AppError>
where
    C: ConnectionTrait,
{
    normalize_video_prompt_kind(&payload.prompt_kind)?;
    ensure_json_object(&payload.params_json, "decision_payload_json.params_json")?;
    if payload.text_anchor_refs.is_empty() {
        return Err(AppError::InvalidArgument(
            "decision_payload_json.text_anchor_refs 不能为空".to_string(),
        ));
    }
    let spec = load_run_generation_spec(db, run, payload.generation_spec_id).await?;
    replace_generation_spec_refs(
        db,
        &spec,
        run.to_snapshot_id,
        payload.text_anchor_refs,
        payload.entity_refs,
        payload.media_refs,
    )
    .await?;
    let mut updated = update_spec_to_snapshot(db, &spec, run.to_snapshot_id).await?;
    let mut active: generation_spec::ActiveModel = updated.into();
    active.params_json = Set(with_prompt_kind(payload.params_json, &payload.prompt_kind)?);
    active.updated_at = Set(Utc::now().naive_utc());
    updated = active.update(db).await.map_err(AppError::Database)?;
    validate_required_prompt_refs(db, &updated).await?;
    Ok(updated)
}

async fn insert_prompt_artifact<C>(
    db: &C,
    generation_spec_id: i64,
    prompt_kind: &str,
    prompt_text: &str,
    renderer_version: &str,
    edited_by_user: bool,
) -> Result<prompt_artifact::Model, AppError>
where
    C: ConnectionTrait,
{
    normalize_video_prompt_kind(prompt_kind)?;
    validate_prompt_artifact_text(prompt_kind, prompt_text)?;
    prompt_artifact::ActiveModel {
        generation_spec_id: Set(generation_spec_id),
        prompt_kind: Set(prompt_kind.to_string()),
        prompt_text: Set(prompt_text.to_string()),
        renderer_version: Set(renderer_version.to_string()),
        edited_by_user: Set(edited_by_user),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

async fn render_and_insert_prompt_artifact<C>(
    db: &C,
    spec: &generation_spec::Model,
    strategy: &str,
) -> Result<prompt_artifact::Model, AppError>
where
    C: ConnectionTrait,
{
    let prompt_kind = required_json_str(&spec.params_json, "prompt_kind")?;
    let prompt_text = render_prompt_text(spec, prompt_kind)?;
    insert_prompt_artifact(
        db,
        spec.id,
        prompt_kind,
        &prompt_text,
        &format!("source-sync-{strategy}-v1"),
        false,
    )
    .await
}

async fn create_replacement_spec<C>(
    db: &C,
    run: &source_sync_run::Model,
    payload: ReplacementSpecPayload,
) -> Result<generation_spec::Model, AppError>
where
    C: ConnectionTrait,
{
    normalize_video_prompt_kind(&payload.prompt_kind)?;
    ensure_json_object(&payload.params_json, "decision_payload_json.params_json")?;
    if payload.text_anchor_refs.is_empty() {
        return Err(AppError::InvalidArgument(
            "decision_payload_json.text_anchor_refs 不能为空".to_string(),
        ));
    }
    let source_spec = load_run_generation_spec(db, run, payload.generation_spec_id).await?;
    if source_spec.target_entity_kind != "segment" {
        return Err(AppError::InvalidArgument(
            "create_replacement 只支持视频片段生成意图".to_string(),
        ));
    }
    let source_chapter_id = source_spec.source_chapter_id.ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "生成意图 {} 缺少 source_chapter_id",
            source_spec.id
        ))
    })?;
    let snapshot_chapter =
        source_snapshot_chapter_for_source(db, run.to_snapshot_id, source_chapter_id).await?;
    validate_replacement_segment_payload(db, &payload.replacement_segment, source_spec.project_id)
        .await?;

    let now = Utc::now().naive_utc();
    let segment = video_segment::ActiveModel {
        project_id: Set(source_spec.project_id),
        root_project_id: Set(source_spec.root_project_id),
        source_chapter_id: Set(Some(source_chapter_id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        segment_type: Set("replacement".to_string()),
        order_index: Set(payload.replacement_segment.order_index),
        title: Set(required_text(
            &payload.replacement_segment.title,
            "replacement_segment.title",
        )?
        .to_string()),
        duration_secs: Set(payload.replacement_segment.duration_secs),
        source_generation_spec_id: Set(None),
        result_media_asset_id: Set(None),
        previous_segment_id: Set(payload.replacement_segment.previous_segment_id),
        next_segment_id: Set(payload.replacement_segment.next_segment_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;

    let snapshot = project_source_snapshot::Entity::find_by_id(run.to_snapshot_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("目标源快照不存在: {}", run.to_snapshot_id)))?;
    let params_json = with_prompt_kind(payload.params_json, &payload.prompt_kind)?;
    let spec = generation_spec::ActiveModel {
        project_id: Set(source_spec.project_id),
        root_project_id: Set(source_spec.root_project_id),
        source_context_kind: Set(Some("snapshot_backed".to_string())),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_chapter_id: Set(Some(source_chapter_id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        target_kind: Set(source_spec.target_kind.clone()),
        target_entity_kind: Set("segment".to_string()),
        target_entity_id: Set(Some(segment.id)),
        status: Set("draft".to_string()),
        source_version_hash: Set(snapshot.source_version_hash),
        params_json: Set(params_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;

    insert_generation_spec_refs(
        db,
        &spec,
        run.to_snapshot_id,
        payload.text_anchor_refs,
        payload.entity_refs,
        payload.media_refs,
    )
    .await?;
    let mut active_segment: video_segment::ActiveModel = segment.into();
    active_segment.source_generation_spec_id = Set(Some(spec.id));
    active_segment.updated_at = Set(now);
    active_segment
        .update(db)
        .await
        .map_err(AppError::Database)?;
    validate_required_prompt_refs(db, &spec).await?;
    Ok(spec)
}

pub(super) fn parse_decision_payload<T>(value: &Value) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value.clone())
        .map_err(|err| AppError::InvalidArgument(format!("同步决策 payload 无效: {err}")))
}

async fn load_run_generation_spec<C>(
    db: &C,
    run: &source_sync_run::Model,
    generation_spec_id: i64,
) -> Result<generation_spec::Model, AppError>
where
    C: ConnectionTrait,
{
    let spec = generation_spec::Entity::find_by_id(generation_spec_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("生成意图不存在: {generation_spec_id}")))?;
    if spec.project_id != run.derived_project_id || spec.root_project_id != run.root_project_id {
        return Err(AppError::InvalidArgument(format!(
            "生成意图 {} 不属于本次源同步运行",
            generation_spec_id
        )));
    }
    Ok(spec)
}

async fn source_snapshot_chapter_for_source<C>(
    db: &C,
    snapshot_id: i64,
    source_chapter_id: i64,
) -> Result<source_snapshot_chapter::Model, AppError>
where
    C: ConnectionTrait,
{
    source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "目标源快照 {} 中不存在源章节 {}",
                snapshot_id, source_chapter_id
            ))
        })
}

async fn replace_generation_spec_refs<C>(
    db: &C,
    spec: &generation_spec::Model,
    snapshot_id: i64,
    text_anchor_refs: Vec<TextAnchorRefInput>,
    entity_refs: Vec<EntityRefInput>,
    media_refs: Vec<MediaRefInput>,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    generation_spec_text_anchor::Entity::delete_many()
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(spec.id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_entity_ref::Entity::delete_many()
        .filter(generation_spec_entity_ref::Column::GenerationSpecId.eq(spec.id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_media_ref::Entity::delete_many()
        .filter(generation_spec_media_ref::Column::GenerationSpecId.eq(spec.id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    insert_generation_spec_refs(
        db,
        spec,
        snapshot_id,
        text_anchor_refs,
        entity_refs,
        media_refs,
    )
    .await
}

async fn insert_generation_spec_refs<C>(
    db: &C,
    spec: &generation_spec::Model,
    snapshot_id: i64,
    text_anchor_refs: Vec<TextAnchorRefInput>,
    entity_refs: Vec<EntityRefInput>,
    media_refs: Vec<MediaRefInput>,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    for item in text_anchor_refs {
        let role = normalize_text_anchor_role(&item.role)?;
        let anchor = text_anchor::Entity::find_by_id(item.text_anchor_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::NotFound(format!("文本锚点不存在: {}", item.text_anchor_id))
            })?;
        if anchor.project_id != spec.root_project_id
            || anchor.source_snapshot_id != Some(snapshot_id)
        {
            return Err(AppError::InvalidArgument(format!(
                "文本锚点 {} 不属于目标根项目和目标快照",
                anchor.id
            )));
        }
        insert_generation_text_anchor_ref(db, spec.id, anchor.id, role).await?;
    }
    for item in entity_refs {
        let entity_kind = normalize_entity_kind(&item.entity_kind)?;
        let role = normalize_entity_role(&item.role)?;
        validate_entity_ref(
            db,
            spec.root_project_id,
            snapshot_id,
            entity_kind,
            item.entity_id,
        )
        .await?;
        insert_generation_entity_ref(db, spec.id, entity_kind, item.entity_id, role).await?;
    }
    for item in media_refs {
        let role = normalize_media_ref_role(&item.role)?;
        insert_generation_media_ref(db, spec, item.media_asset_id, role).await?;
    }
    Ok(())
}

async fn ensure_spec_anchors_belong_to_snapshot<C>(
    db: &C,
    generation_spec_id: i64,
    snapshot_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let refs = generation_spec_text_anchor::Entity::find()
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(generation_spec_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if refs.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "生成意图 {} 缺少文本锚点引用",
            generation_spec_id
        )));
    }
    for item in refs {
        let anchor = text_anchor::Entity::find_by_id(item.text_anchor_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::NotFound(format!("文本锚点不存在: {}", item.text_anchor_id))
            })?;
        if anchor.source_snapshot_id != Some(snapshot_id) {
            return Err(AppError::InvalidArgument(format!(
                "生成意图 {} 仍引用非目标快照文本锚点 {}",
                generation_spec_id, anchor.id
            )));
        }
    }
    Ok(())
}

async fn validate_replacement_segment_payload<C>(
    db: &C,
    payload: &ReplacementSegmentPayload,
    project_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    required_text(&payload.title, "replacement_segment.title")?;
    if payload.duration_secs <= 0.0 {
        return Err(AppError::InvalidArgument(
            "replacement_segment.duration_secs 必须大于 0".to_string(),
        ));
    }
    if let Some(previous_segment_id) = payload.previous_segment_id {
        ensure_video_segment_in_project(db, previous_segment_id, project_id).await?;
    }
    if let Some(next_segment_id) = payload.next_segment_id {
        ensure_video_segment_in_project(db, next_segment_id, project_id).await?;
    }
    Ok(())
}

async fn ensure_video_segment_in_project<C>(
    db: &C,
    segment_id: i64,
    project_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let count = video_segment::Entity::find_by_id(segment_id)
        .filter(video_segment::Column::ProjectId.eq(project_id))
        .count(db)
        .await
        .map_err(AppError::Database)?;
    if count == 0 {
        return Err(AppError::InvalidArgument(format!(
            "视频片段 {} 不属于视频项目 {}",
            segment_id, project_id
        )));
    }
    Ok(())
}

async fn delete_local_generation_spec<C>(
    db: &C,
    run: &source_sync_run::Model,
    generation_spec_id: i64,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let spec = load_run_generation_spec(db, run, generation_spec_id).await?;
    let segments = video_segment::Entity::find()
        .filter(video_segment::Column::SourceGenerationSpecId.eq(spec.id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    for segment in &segments {
        if segment.result_media_asset_id.is_some() {
            return Err(AppError::InvalidArgument(format!(
                "视频片段 {} 已有结果媒体，不能 delete_local",
                segment.id
            )));
        }
        let clip_count = assembly_clip::Entity::find()
            .filter(assembly_clip::Column::SourceSegmentId.eq(segment.id))
            .count(db)
            .await
            .map_err(AppError::Database)?;
        if clip_count > 0 {
            return Err(AppError::InvalidArgument(format!(
                "视频片段 {} 已进入时间线，不能 delete_local",
                segment.id
            )));
        }
    }
    for segment in segments {
        video_segment::Entity::delete_by_id(segment.id)
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }
    delete_generation_specs(db, &[spec.id]).await
}

fn ensure_clean_sync_target(
    conflict: &source_sync_conflict::Model,
    strategy: &str,
) -> Result<(), AppError> {
    if matches!(conflict.local_state_kind.as_str(), "clean" | "draft") {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{strategy} 不能处理本地状态为 {} 的对象，请使用保留当前、手动合并或创建替换版本",
            conflict.local_state_kind
        )))
    }
}

fn with_prompt_kind(mut params_json: Value, prompt_kind: &str) -> Result<Value, AppError> {
    normalize_video_prompt_kind(prompt_kind)?;
    let object = params_json.as_object_mut().ok_or_else(|| {
        AppError::InvalidArgument("decision_payload_json.params_json 必须是 JSON 对象".to_string())
    })?;
    object.insert(
        "prompt_kind".to_string(),
        Value::String(prompt_kind.to_string()),
    );
    Ok(params_json)
}

fn required_json_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, AppError> {
    let text = value.get(field).and_then(Value::as_str).ok_or_else(|| {
        AppError::InvalidArgument(format!("params_json.{field} 必须是非空字符串"))
    })?;
    required_text(text, &format!("params_json.{field}"))
}

fn ensure_json_object(value: &Value, label: &str) -> Result<(), AppError> {
    if value.is_object() {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{label} 必须是 JSON 对象"
        )))
    }
}
