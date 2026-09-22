use a4n_db_models::models::generation::workflow_run;
use a4n_domain::shooting_script::{
    ChapterShootingScriptUnitProgressCheckpoint, EditChapterShootingScriptProgressInput,
    EditedChapterShootingScriptProgressView,
};
use a4n_domain::source_text_hash;
use a4n_domain::workflow::{WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set, TransactionTrait};

use super::mutations::validate_director_constraints;
use super::validation::require_positive;
use super::{ChapterVisualPlanIdentity, ShootingScriptDbError};

pub async fn edit_chapter_shooting_script_progress(
    db: &DatabaseConnection,
    input: EditChapterShootingScriptProgressInput,
) -> Result<EditedChapterShootingScriptProgressView, ShootingScriptDbError> {
    require_positive(input.project_id, "project_id")?;
    require_positive(input.source_chapter_id, "source_chapter_id")?;
    require_positive(input.workflow_run_id, "workflow_run_id")?;
    let tx = db.begin().await?;
    let source =
        crate::project::resolve_chapter_source(&tx, input.project_id, input.source_chapter_id)
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
    if checkpoint.version != 1
        || checkpoint.progress.workflow_run_id != run.id
        || checkpoint.source_content_sha256 != source.content_sha256
        || checkpoint.progress.completed_record.is_some()
    {
        return Err(ShootingScriptDbError::InvalidState(
            "拍摄剧本进度与当前来源不一致，请重新加载".into(),
        ));
    }
    let current = &checkpoint.progress;
    current
        .script
        .validate_partial(current.completed_unit_count, current.total_unit_count)
        .map_err(|error| ShootingScriptDbError::InvalidState(error.to_string()))?;
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
    let invalidated_project_ids = crate::project::projects_using_source_snapshot(
        &tx,
        source.root_project_id,
        source.source_snapshot_id,
    )
    .await?;
    tx.commit().await?;
    Ok(EditedChapterShootingScriptProgressView {
        progress: checkpoint.progress,
        invalidated_project_ids,
    })
}
