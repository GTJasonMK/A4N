use a4n_db_models::models::{chapter::chapter_visual_plan, generation::workflow_run};
use a4n_domain::shooting_script::{
    ChapterShootingScriptUnitProgressCheckpoint, ChapterShootingScriptUnitProgressView,
    EditChapterShootingScriptProgressInput, EditedChapterShootingScriptProgressView,
};
use a4n_domain::source_text_hash;
use a4n_domain::workflow::{
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use std::collections::HashMap;

use super::mutations::validate_director_constraints;
use super::validation::{require_positive, validate_source};
use super::{ChapterVisualPlanIdentity, ShootingScriptDbError};

pub async fn chapter_shooting_script_progress_for_source<C: ConnectionTrait>(
    db: &C,
    source: &a4n_domain::chapter::ResolvedChapterSource,
) -> Result<Option<ChapterShootingScriptUnitProgressView>, ShootingScriptDbError> {
    validate_source(source)?;
    let Some(confirmed) =
        crate::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(db, source)
            .await
            .map_err(super::plan_rebase::visual_plan_error)?
    else {
        return Ok(None);
    };
    Ok(chapter_shooting_script_progress_for_plans(
        db,
        source.requested_project_id,
        &[&confirmed.record],
    )
    .await?
    .remove(&source.source_chapter_id)
    .map(|(_, progress)| progress))
}

/// Only progress owned by this project and the exact confirmed plan is eligible.
async fn chapter_shooting_script_progress_for_plans<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    plans: &[&chapter_visual_plan::Model],
) -> Result<HashMap<i64, (String, ChapterShootingScriptUnitProgressView)>, ShootingScriptDbError> {
    if plans.is_empty() {
        return Ok(HashMap::new());
    }
    let plan_condition = plans.iter().fold(Condition::any(), |condition, plan| {
        condition.add(
            Condition::all()
                .add(workflow_run::Column::ChapterId.eq(plan.source_chapter_id))
                .add(Expr::cust_with_values(
                    "input_fingerprint_json ->> 'visual_plan_id' = $1",
                    [plan.id.to_string()],
                )),
        )
    });
    let plans_by_chapter = plans
        .iter()
        .map(|plan| (plan.source_chapter_id, *plan))
        .collect::<HashMap<_, _>>();
    let mut result = HashMap::new();
    for run in workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::WorkflowKind.eq("chapter_shooting_script"))
        .filter(workflow_run::Column::CanResume.eq(true))
        .filter(workflow_run::Column::Status.is_in([
            WORKFLOW_STATUS_RUNNING,
            WORKFLOW_STATUS_INTERRUPTED,
            WORKFLOW_STATUS_FAILED,
        ]))
        .filter(plan_condition)
        .distinct_on([workflow_run::Column::ChapterId])
        .order_by_asc(workflow_run::Column::ChapterId)
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .all(db)
        .await?
    {
        let Some(snapshot) = run
            .progress_json
            .as_ref()
            .and_then(|progress| progress.get("snapshot"))
        else {
            continue;
        };
        let checkpoint: ChapterShootingScriptUnitProgressCheckpoint =
            serde_json::from_value(snapshot.clone()).map_err(|error| {
                ShootingScriptDbError::InvalidState(format!("拍摄剧本 snapshot 无效: {error}"))
            })?;
        checkpoint
            .validate_for_run(run.id)
            .map_err(|error| ShootingScriptDbError::InvalidState(error.to_string()))?;
        let chapter_id = run.chapter_id.ok_or_else(|| {
            ShootingScriptDbError::InvalidState("拍摄剧本工作流缺少章节身份".into())
        })?;
        let plan = plans_by_chapter[&chapter_id];
        let identity = checkpoint
            .progress
            .script
            .visual_plan_source
            .as_ref()
            .ok_or_else(|| {
                ShootingScriptDbError::InvalidState("拍摄剧本进度缺少导演编排身份".into())
            })?;
        if checkpoint.source_content_sha256 != plan.source_content_sha256
            || identity.plan_id != plan.id
            || identity.plan_version != plan.plan_version
            || identity.plan_sha256 != plan.plan_sha256
        {
            continue;
        }
        let fingerprint = run.input_fingerprint_json.as_ref().ok_or_else(|| {
            ShootingScriptDbError::InvalidState("拍摄剧本工作流缺少输入指纹".into())
        })?;
        let fingerprint = a4n_domain::generation::canonical_json_sha256(fingerprint)
            .map_err(|error| ShootingScriptDbError::InvalidState(error.to_string()))?;
        result.insert(chapter_id, (fingerprint, checkpoint.progress));
    }
    Ok(result)
}

pub async fn edit_chapter_shooting_script_progress(
    db: &DatabaseConnection,
    input: EditChapterShootingScriptProgressInput,
) -> Result<EditedChapterShootingScriptProgressView, ShootingScriptDbError> {
    require_positive(input.project_id, "project_id")?;
    require_positive(input.source_chapter_id, "source_chapter_id")?;
    require_positive(input.workflow_run_id, "workflow_run_id")?;
    let tx = db.begin().await?;
    let source = crate::project::resolve_video_chapter_source(
        &tx,
        input.project_id,
        input.source_chapter_id,
    )
    .await
    .map_err(|error| match error {
        crate::project::ResolveChapterSourceError::Database(error) => {
            ShootingScriptDbError::Database(error)
        }
        crate::project::ResolveChapterSourceError::NotFound(message) => {
            ShootingScriptDbError::NotFound(message)
        }
        crate::project::ResolveChapterSourceError::InvalidArgument(message) => {
            ShootingScriptDbError::InvalidArgument(message)
        }
        crate::project::ResolveChapterSourceError::InvalidState(message) => {
            ShootingScriptDbError::InvalidState(message)
        }
    })?;
    let content = crate::chapter_content::chapter_by_id_for_analysis(&tx, input.source_chapter_id)
        .await
        .map_err(|error| ShootingScriptDbError::InvalidState(error.to_string()))?
        .content;
    if source.source_snapshot_id.is_none() && source_text_hash(&content) != source.content_sha256 {
        return Err(ShootingScriptDbError::InvalidState(
            "章节正文已变化，请重新加载".into(),
        ));
    }
    let run = crate::workflow::locked_workflow_run_by_id(&tx, input.workflow_run_id)
        .await
        .map_err(|error| match error {
            crate::workflow::WorkflowReadError::Database(error) => {
                ShootingScriptDbError::Database(error)
            }
            crate::workflow::WorkflowReadError::NotFound(message) => {
                ShootingScriptDbError::NotFound(message)
            }
            crate::workflow::WorkflowReadError::InvalidArgument(message) => {
                ShootingScriptDbError::InvalidArgument(message)
            }
            crate::workflow::WorkflowReadError::InvalidData(message) => {
                ShootingScriptDbError::InvalidState(message)
            }
        })?;
    if run.workflow_kind != "chapter_shooting_script"
        || run.project_id != Some(input.project_id)
        || run.chapter_id != Some(input.source_chapter_id)
    {
        return Err(ShootingScriptDbError::InvalidArgument(
            "拍摄剧本进度不属于当前项目和章节".into(),
        ));
    }
    if !run.can_resume
        || !matches!(
            run.status.as_str(),
            WORKFLOW_STATUS_INTERRUPTED | WORKFLOW_STATUS_FAILED
        )
    {
        return Err(ShootingScriptDbError::InvalidState(
            "只能编辑已暂停或失败的可恢复拍摄剧本，请先等待当前生成结束".into(),
        ));
    }
    let mut progress_json = run
        .progress_json
        .clone()
        .ok_or_else(|| ShootingScriptDbError::InvalidState("拍摄剧本没有已保存的进度".into()))?;
    let snapshot = progress_json
        .get("snapshot")
        .ok_or_else(|| ShootingScriptDbError::InvalidState("拍摄剧本进度缺少 snapshot".into()))?;
    let mut checkpoint: ChapterShootingScriptUnitProgressCheckpoint =
        serde_json::from_value(snapshot.clone()).map_err(|error| {
            ShootingScriptDbError::InvalidState(format!("拍摄剧本 snapshot 无效: {error}"))
        })?;
    checkpoint
        .validate_for_run(run.id)
        .map_err(|error| ShootingScriptDbError::InvalidState(error.to_string()))?;
    if checkpoint.source_content_sha256 != source.content_sha256 {
        return Err(ShootingScriptDbError::InvalidState(
            "拍摄剧本进度与当前来源不一致，请重新加载".into(),
        ));
    }
    let current = &checkpoint.progress;
    if current.script != input.expected_script {
        return Err(ShootingScriptDbError::InvalidState(
            "拍摄剧本进度已发生变化，请重新加载后保存".into(),
        ));
    }
    input
        .script
        .validate_partial(current.completed_unit_count, current.total_unit_count)
        .map_err(|error| ShootingScriptDbError::InvalidArgument(error.to_string()))?;
    if input.script.visual_plan_source != current.script.visual_plan_source {
        return Err(ShootingScriptDbError::InvalidArgument(
            "编辑不能改变拍摄剧本的导演方案身份".into(),
        ));
    }
    for scene in &input.script.scenes {
        let original = current
            .script
            .scenes
            .iter()
            .find(|original| original.narrative_unit_index == scene.narrative_unit_index)
            .ok_or_else(|| {
                ShootingScriptDbError::InvalidArgument("编辑只能修改或删除已经完成的场景".into())
            })?;
        if scene.scene_identity != original.scene_identity
            || scene.shots.iter().any(|shot| {
                !original
                    .shots
                    .iter()
                    .any(|original| original.shot_id == shot.shot_id)
            })
        {
            return Err(ShootingScriptDbError::InvalidArgument(
                "编辑不能替换场景或镜头的来源身份".into(),
            ));
        }
    }
    let identity =
        input.script.visual_plan_source.as_ref().ok_or_else(|| {
            ShootingScriptDbError::InvalidArgument("拍摄剧本缺少导演方案身份".into())
        })?;
    validate_director_constraints(
        &tx,
        (
            source.requested_project_id,
            source.root_project_id,
            source.source_chapter_id,
            source.source_snapshot_id,
            source.source_snapshot_chapter_id,
        ),
        &ChapterVisualPlanIdentity {
            id: identity.plan_id,
            version: identity.plan_version,
            sha256: identity.plan_sha256.clone(),
        },
        &input.script,
        Some(&current.script),
    )
    .await?;
    if input.script == current.script {
        tx.commit().await?;
        return Ok(EditedChapterShootingScriptProgressView {
            progress: checkpoint.progress,
            invalidated_project_ids: Vec::new(),
        });
    }
    checkpoint.progress.script = input.script;
    progress_json["snapshot"] = serde_json::to_value(&checkpoint)
        .map_err(|error| ShootingScriptDbError::InvalidState(error.to_string()))?;
    let mut active: workflow_run::ActiveModel = run.into();
    active.progress_json = Set(Some(progress_json));
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(&tx).await?;
    let invalidated_project_ids = vec![source.requested_project_id];
    tx.commit().await?;
    Ok(EditedChapterShootingScriptProgressView {
        progress: checkpoint.progress,
        invalidated_project_ids,
    })
}
