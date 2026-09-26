use a4n_db_models::models::chapter::chapter_shooting_script;
use a4n_domain::shooting_script::{
    ShootingScriptPlanSyncV1, ShootingVisualPlanSourceV1, project_visual_plan_for_shooting_script,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::{
    codec::{parse_record, stored_visual_plan_identity},
    contracts::{
        ChapterVisualPlanIdentity, ShootingScriptDbError, UpdateChapterShootingScriptInput,
    },
    mutations::update_chapter_shooting_script,
};
use crate::chapter_visual_plan::ParsedChapterVisualPlan;

/// 在确认导演编排的同一事务内，把同一正文版本的拍摄剧本变基到新方案。
///
/// 只有内容变化的镜头被标记为过时，其余镜头与场景原样沿用；返回变基后的逐镜头同步状态。
/// 没有拍摄剧本或拍摄剧本未绑定导演编排时返回 None。
pub(crate) async fn rebase_chapter_shooting_script_onto_plan<C>(
    db: &C,
    plan: &ParsedChapterVisualPlan,
) -> Result<Option<ShootingScriptPlanSyncV1>, ShootingScriptDbError>
where
    C: ConnectionTrait,
{
    let Some(record) = chapter_shooting_script::Entity::find()
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
        return Ok(Some(current.script.plan_sync(&director)));
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
    Ok(Some(sync))
}
