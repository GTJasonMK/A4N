use a4n_db_models::models::chapter::chapter_shooting_script;
use a4n_domain::shooting_script::{
    ChapterShootingScriptV2, DirectorScriptPlan, ShootingScriptPlanSyncV1,
    ShootingVisualPlanSourceV1, project_visual_plan_for_shooting_script,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::{
    codec::{parse_record, stored_visual_plan_identity},
    contracts::{
        ChapterVisualPlanIdentity, ParsedChapterShootingScript, ShootingScriptDbError,
        UpdateChapterShootingScriptInput,
    },
    mutations::update_chapter_shooting_script,
};
use crate::chapter_visual_plan::ParsedChapterVisualPlan;

pub async fn chapter_shooting_script_plan_sync<C: ConnectionTrait>(
    db: &C,
    script: &ParsedChapterShootingScript,
) -> Result<ShootingScriptPlanSyncV1, ShootingScriptDbError> {
    Ok(script
        .script
        .plan_sync(&shooting_script_director_plan(db, &script.script).await?))
}

pub(crate) async fn shooting_script_director_plan<C: ConnectionTrait>(
    db: &C,
    script: &ChapterShootingScriptV2,
) -> Result<DirectorScriptPlan, ShootingScriptDbError> {
    let identity = script
        .visual_plan_source
        .as_ref()
        .ok_or_else(|| ShootingScriptDbError::InvalidState("拍摄剧本未绑定导演编排".into()))?;
    let plan = crate::chapter_visual_plan::chapter_visual_plan_by_id(db, identity.plan_id)
        .await
        .map_err(visual_plan_error)?;
    project_visual_plan_for_shooting_script(&plan.plan, &[])
        .map_err(|error| ShootingScriptDbError::InvalidState(error.to_string()))
}

pub(super) fn visual_plan_error(
    error: crate::chapter_visual_plan::ChapterVisualPlanDbError,
) -> ShootingScriptDbError {
    match error {
        crate::chapter_visual_plan::ChapterVisualPlanDbError::Database(error) => {
            ShootingScriptDbError::Database(error)
        }
        error => ShootingScriptDbError::InvalidArgument(error.to_string()),
    }
}

/// 在确认导演编排的同一事务内，把同一正文版本的拍摄剧本变基到新方案。
///
/// 只有内容变化的镜头被标记为过时，其余镜头与场景原样沿用；返回变基后的逐镜头同步状态。
/// 没有拍摄剧本或拍摄剧本未绑定导演编排时返回 None。
pub(crate) async fn rebase_chapter_shooting_script_onto_plan<C>(
    db: &C,
    plan: &ParsedChapterVisualPlan,
) -> Result<Option<(ShootingScriptPlanSyncV1, bool)>, ShootingScriptDbError>
where
    C: ConnectionTrait,
{
    let Some(record) = chapter_shooting_script::Entity::find()
        .filter(chapter_shooting_script::Column::ProjectId.eq(plan.record.project_id))
        .filter(chapter_shooting_script::Column::RootProjectId.eq(plan.record.root_project_id))
        .filter(chapter_shooting_script::Column::SourceChapterId.eq(plan.record.source_chapter_id))
        .filter(
            chapter_shooting_script::Column::SourceRevisionKey.eq(&plan.record.source_revision_key),
        )
        .filter(
            chapter_shooting_script::Column::SourceContentSha256
                .eq(&plan.record.source_content_sha256),
        )
        .order_by_desc(chapter_shooting_script::Column::UpdatedAt)
        .order_by_desc(chapter_shooting_script::Column::Id)
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    if stored_visual_plan_identity(&record)?.is_none() {
        return Ok(None);
    }
    let director = project_visual_plan_for_shooting_script(&plan.plan, &[])
        .map_err(|error| ShootingScriptDbError::InvalidState(error.to_string()))?;
    let current = parse_record(record)?;
    if current.record.visual_plan_id == Some(plan.record.id) {
        return Ok(Some((current.script.plan_sync(&director), false)));
    }
    let identity = ChapterVisualPlanIdentity {
        id: plan.record.id,
        version: plan.record.plan_version,
        sha256: plan.record.plan_sha256.clone(),
    };
    let rebased = current
        .script
        .rebased_onto(
            &director,
            ShootingVisualPlanSourceV1 {
                plan_id: identity.id,
                plan_version: identity.version,
                plan_sha256: identity.sha256.clone(),
            },
        )
        .map_err(|error| {
            ShootingScriptDbError::InvalidState(format!(
                "拍摄剧本 {} 无法同步到导演编排 v{}: {error}",
                current.record.id, identity.version
            ))
        })?;
    let sync = rebased.plan_sync(&director);
    let previous_director = shooting_script_director_plan(db, &current.script).await?;
    let fingerprints = |script: &ChapterShootingScriptV2, plan: &DirectorScriptPlan| {
        script
            .shot_content_fingerprints(plan)
            .map_err(|error| ShootingScriptDbError::InvalidState(error.to_string()))
    };
    let changed = !sync.is_current()
        || fingerprints(&current.script, &previous_director)? != fingerprints(&rebased, &director)?;
    update_chapter_shooting_script(
        db,
        UpdateChapterShootingScriptInput {
            record_id: current.record.id,
            expected_revision: current.record.revision,
            input_fingerprint_sha256: current.record.input_fingerprint_sha256,
            visual_plan: identity,
            script: rebased,
            workflow_run_id: current.record.workflow_run_id,
        },
    )
    .await?;
    Ok(Some((sync, changed)))
}
