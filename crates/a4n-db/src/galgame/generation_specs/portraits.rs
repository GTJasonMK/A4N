use super::*;

pub async fn galgame_webgal_portraits(
    db: &DatabaseConnection,
    root_project_id: i64,
) -> Result<Vec<character_portrait::Model>, GalgameGenerationRecordError> {
    validate_galgame_generation_id(root_project_id, "root_project_id")?;
    character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(root_project_id))
        .order_by_asc(character_portrait::Column::Id)
        .all(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)
}

pub async fn galgame_portrait_spec_target_profile(
    db: &DatabaseConnection,
    spec: &generation_spec::Model,
    input: &GalgamePortraitSpecTargetInput,
) -> Result<protagonist_profile::Model, GalgameGenerationRecordError> {
    validate_galgame_portrait_spec_target_input(spec, input)?;
    let project = novel_project::Entity::find_by_id(spec.project_id)
        .one(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .ok_or_else(|| {
            GalgameGenerationRecordError::NotFound(format!("项目不存在: {}", spec.project_id))
        })?;
    if project.project_kind != PROJECT_KIND_GALGAME {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分生成意图不属于 Galgame 项目: spec_id={}, project_id={}",
            spec.id, project.id
        )));
    }
    let root_project_id = project.root_project_id.ok_or_else(|| {
        GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 项目缺少根文本项目: {}",
            project.id
        ))
    })?;
    let source_snapshot_id = project.source_snapshot_id.ok_or_else(|| {
        GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 项目缺少源文本快照: {}",
            project.id
        ))
    })?;
    validate_galgame_portrait_spec_family_context(spec, root_project_id, source_snapshot_id)?;
    validate_galgame_portrait_spec_source_chapter(spec)?;
    let profile = protagonist_profile::Entity::find_by_id(input.profile_id)
        .one(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .ok_or_else(|| {
            GalgameGenerationRecordError::NotFound(format!(
                "Galgame 立绘差分角色档案不存在: {}",
                input.profile_id
            ))
        })?;
    if profile.project_id != root_project_id {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分角色档案不属于当前 Galgame 根项目: spec_id={}, profile_id={}",
            spec.id, profile.id
        )));
    }
    let snapshot_profile = source_snapshot_profile::Entity::find()
        .filter(source_snapshot_profile::Column::SnapshotId.eq(source_snapshot_id))
        .filter(source_snapshot_profile::Column::RootProjectId.eq(root_project_id))
        .filter(source_snapshot_profile::Column::ProfileKind.eq("character"))
        .filter(source_snapshot_profile::Column::SourceProfileId.eq(profile.id))
        .one(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?;
    if snapshot_profile.is_none() {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分角色档案不属于源快照: spec_id={}, profile_id={}, source_snapshot_id={}",
            spec.id, profile.id, source_snapshot_id
        )));
    }
    validate_galgame_portrait_spec_source_lines(
        db,
        spec,
        input,
        &project,
        root_project_id,
        source_snapshot_id,
    )
    .await?;
    Ok(profile)
}

pub(in crate::galgame) fn validate_galgame_portrait_spec_target_input(
    spec: &generation_spec::Model,
    input: &GalgamePortraitSpecTargetInput,
) -> Result<(), GalgameGenerationRecordError> {
    validate_galgame_generation_id(spec.id, "generation_spec_id")?;
    validate_galgame_generation_id(spec.project_id, "project_id")?;
    validate_galgame_generation_id(input.profile_id, "profile_id")?;
    require_galgame_generation_text(&input.expression, "expression")?;
    if let Some(pose) = input.pose.as_deref() {
        require_galgame_generation_text(pose, "pose")?;
    }
    if let Some(line_id) = input.source_line_ids.iter().find(|line_id| **line_id <= 0) {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "source_line_ids 必须全部是正整数: {line_id}"
        )));
    }
    Ok(())
}

pub async fn completed_galgame_portrait_record(
    db: &DatabaseConnection,
    spec: &generation_spec::Model,
    profile_id: i64,
    expression: &str,
    pose: Option<&str>,
) -> Result<character_portrait::Model, GalgameGenerationRecordError> {
    validate_galgame_generation_id(spec.id, "generation_spec_id")?;
    validate_galgame_generation_id(spec.root_project_id, "root_project_id")?;
    validate_galgame_generation_id(profile_id, "profile_id")?;
    let expression = require_galgame_generation_text(expression, "expression")?;
    let pose = pose
        .map(|pose| require_galgame_generation_text(pose, "pose"))
        .transpose()?;
    let mut query = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(spec.root_project_id))
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .filter(character_portrait::Column::IsPrimary.eq(false))
        .filter(character_portrait::Column::Expression.eq(expression));
    query = match pose {
        Some(pose) => query.filter(character_portrait::Column::Pose.eq(pose)),
        None => query.filter(character_portrait::Column::Pose.is_null()),
    };
    query
        .order_by_desc(character_portrait::Column::Id)
        .one(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .ok_or_else(|| {
            GalgameGenerationRecordError::NotFound(format!(
                "Galgame 立绘差分生成意图已完成但缺少可用立绘: spec_id={}, profile_id={}, expression={}, pose={:?}",
                spec.id, profile_id, expression, pose
            ))
        })
}

pub async fn primary_galgame_portrait_reference_image_ids(
    db: &DatabaseConnection,
    profile: &protagonist_profile::Model,
) -> Result<Vec<i64>, GalgameGenerationRecordError> {
    validate_galgame_generation_id(profile.id, "profile_id")?;
    validate_galgame_generation_id(profile.project_id, "project_id")?;
    let primary = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(profile.project_id))
        .filter(character_portrait::Column::ProfileId.eq(profile.id))
        .filter(character_portrait::Column::IsPrimary.eq(true))
        .order_by_desc(character_portrait::Column::Id)
        .one(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .ok_or_else(|| {
            GalgameGenerationRecordError::NotFound(format!("角色 {} 没有主立绘", profile.id))
        })?;
    let image = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(profile.project_id))
        .filter(generated_image::Column::FilePath.eq(&primary.image_path))
        .order_by_desc(generated_image::Column::Id)
        .one(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .ok_or_else(|| {
            GalgameGenerationRecordError::NotFound(format!(
                "主立绘 {} 缺少可用图片镜像记录",
                primary.id
            ))
        })?;
    Ok(vec![image.id])
}
