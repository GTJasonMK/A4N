use a4n_db_models::models::chapter::chapter_shooting_script;
use a4n_domain::shooting_script::CHAPTER_SHOOTING_SCRIPT_V1;
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    Set,
};

use super::{
    codec::{parse_record, serialize_script, validate_script_visual_plan_identity},
    contracts::{
        InsertChapterShootingScriptInput, ParsedChapterShootingScript, ShootingScriptDbError,
        UpdateChapterShootingScriptInput,
    },
    queries::chapter_shooting_script_by_id,
    validation::{require_positive, validate_sha256, validate_source, validate_workflow_run_id},
};

pub async fn insert_chapter_shooting_script<C>(
    db: &C,
    input: InsertChapterShootingScriptInput,
) -> Result<ParsedChapterShootingScript, ShootingScriptDbError>
where
    C: ConnectionTrait,
{
    validate_source(&input.source)?;
    validate_sha256(&input.input_fingerprint_sha256, "input_fingerprint_sha256")?;
    validate_visual_plan_identity(&input.visual_plan)?;
    validate_workflow_run_id(input.workflow_run_id)?;
    validate_script_visual_plan_identity(&input.script, &input.visual_plan)?;
    validate_director_constraints(
        db,
        (
            input.source.root_project_id,
            input.source.source_chapter_id,
            input.source.source_snapshot_id,
            input.source.source_snapshot_chapter_id,
        ),
        &input.visual_plan,
        &input.script,
        None,
    )
    .await?;
    let (script_json, script_sha256) = serialize_script(&input.script)?;
    let now = Utc::now().naive_utc();
    let record = chapter_shooting_script::ActiveModel {
        id: NotSet,
        root_project_id: Set(input.source.root_project_id),
        source_chapter_id: Set(input.source.source_chapter_id),
        source_snapshot_id: Set(input.source.source_snapshot_id),
        source_snapshot_chapter_id: Set(input.source.source_snapshot_chapter_id),
        source_revision_key: Set(input.source.source_revision_key),
        source_content_sha256: Set(input.source.content_sha256),
        visual_plan_id: Set(Some(input.visual_plan.id)),
        visual_plan_version: Set(Some(input.visual_plan.version)),
        visual_plan_sha256: Set(Some(input.visual_plan.sha256)),
        input_fingerprint_sha256: Set(input.input_fingerprint_sha256),
        contract_version: Set(CHAPTER_SHOOTING_SCRIPT_V1.to_string()),
        revision: Set(1),
        script_sha256: Set(script_sha256),
        script_json: Set(script_json),
        workflow_run_id: Set(input.workflow_run_id),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;
    parse_record(record)
}

pub async fn update_chapter_shooting_script<C>(
    db: &C,
    input: UpdateChapterShootingScriptInput,
) -> Result<ParsedChapterShootingScript, ShootingScriptDbError>
where
    C: ConnectionTrait,
{
    require_positive(input.record_id, "record_id")?;
    if input.expected_revision <= 0 {
        return Err(ShootingScriptDbError::InvalidArgument(
            "expected_revision 必须大于 0".to_string(),
        ));
    }
    validate_sha256(&input.input_fingerprint_sha256, "input_fingerprint_sha256")?;
    validate_visual_plan_identity(&input.visual_plan)?;
    validate_workflow_run_id(input.workflow_run_id)?;
    validate_script_visual_plan_identity(&input.script, &input.visual_plan)?;
    let (script_json, script_sha256) = serialize_script(&input.script)?;
    let current = chapter_shooting_script::Entity::find()
        .filter(chapter_shooting_script::Column::Id.eq(input.record_id))
        .filter(chapter_shooting_script::Column::Revision.eq(input.expected_revision))
        .one(db)
        .await?
        .ok_or(ShootingScriptDbError::RevisionConflict {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
        })?;
    let previous = parse_record(current.clone())?.script;
    validate_director_constraints(
        db,
        (
            current.root_project_id,
            current.source_chapter_id,
            current.source_snapshot_id,
            current.source_snapshot_chapter_id,
        ),
        &input.visual_plan,
        &input.script,
        Some(&previous),
    )
    .await?;
    if current.script_sha256 == script_sha256
        && current.input_fingerprint_sha256 == input.input_fingerprint_sha256
        && current.visual_plan_id == Some(input.visual_plan.id)
        && current.visual_plan_version == Some(input.visual_plan.version)
        && current.visual_plan_sha256 == Some(input.visual_plan.sha256.clone())
    {
        return parse_record(current);
    }

    let next_revision = input.expected_revision.checked_add(1).ok_or_else(|| {
        ShootingScriptDbError::InvalidState("拍摄剧本 revision 超出 i32 范围".to_string())
    })?;
    let updated = chapter_shooting_script::Entity::update_many()
        .col_expr(
            chapter_shooting_script::Column::InputFingerprintSha256,
            Expr::value(input.input_fingerprint_sha256),
        )
        .col_expr(
            chapter_shooting_script::Column::VisualPlanId,
            Expr::value(Some(input.visual_plan.id)),
        )
        .col_expr(
            chapter_shooting_script::Column::VisualPlanVersion,
            Expr::value(Some(input.visual_plan.version)),
        )
        .col_expr(
            chapter_shooting_script::Column::VisualPlanSha256,
            Expr::value(Some(input.visual_plan.sha256)),
        )
        .col_expr(
            chapter_shooting_script::Column::Revision,
            Expr::value(next_revision),
        )
        .col_expr(
            chapter_shooting_script::Column::ScriptSha256,
            Expr::value(script_sha256),
        )
        .col_expr(
            chapter_shooting_script::Column::ScriptJson,
            Expr::value(script_json),
        )
        .col_expr(
            chapter_shooting_script::Column::WorkflowRunId,
            Expr::value(input.workflow_run_id),
        )
        .col_expr(
            chapter_shooting_script::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(chapter_shooting_script::Column::Id.eq(input.record_id))
        .filter(chapter_shooting_script::Column::Revision.eq(input.expected_revision))
        .exec(db)
        .await?;
    if updated.rows_affected != 1 {
        return Err(ShootingScriptDbError::RevisionConflict {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
        });
    }
    chapter_shooting_script_by_id(db, input.record_id).await
}

fn validate_visual_plan_identity(
    identity: &super::contracts::ChapterVisualPlanIdentity,
) -> Result<(), ShootingScriptDbError> {
    if identity.id <= 0 || identity.version <= 0 {
        return Err(ShootingScriptDbError::InvalidArgument(
            "visual plan id 和 version 必须大于 0".into(),
        ));
    }
    validate_sha256(&identity.sha256, "visual_plan_sha256")
}

pub(super) async fn validate_director_constraints<C: ConnectionTrait>(
    db: &C,
    source: (i64, i64, Option<i64>, Option<i64>),
    identity: &super::contracts::ChapterVisualPlanIdentity,
    script: &a4n_domain::shooting_script::ChapterShootingScriptV1,
    previous: Option<&a4n_domain::shooting_script::ChapterShootingScriptV1>,
) -> Result<(), ShootingScriptDbError> {
    let plan = crate::chapter_visual_plan::chapter_visual_plan_by_id(db, identity.id)
        .await
        .map_err(|error| match error {
            crate::chapter_visual_plan::ChapterVisualPlanDbError::Database(error) => {
                ShootingScriptDbError::Database(error)
            }
            error => ShootingScriptDbError::InvalidArgument(error.to_string()),
        })?;
    if (
        plan.record.root_project_id,
        plan.record.source_chapter_id,
        plan.record.source_snapshot_id,
        plan.record.source_snapshot_chapter_id,
    ) != source
        || plan.status != a4n_domain::chapter_visual_plan::ChapterVisualPlanStatus::Confirmed
        || plan.record.plan_version != identity.version
        || plan.record.plan_sha256 != identity.sha256
    {
        return Err(ShootingScriptDbError::InvalidArgument(
            "导演方案与拍摄剧本来源不一致".into(),
        ));
    }
    script
        .validate_director_plan(&plan.plan, previous)
        .map_err(|error| ShootingScriptDbError::InvalidArgument(error.to_string()))
}
