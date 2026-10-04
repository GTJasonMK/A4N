use super::*;
use a4n_domain::{character::CharacterPortraitRecord, generation::GenerationSpecRecord};
use sea_orm::QuerySelect;
use std::collections::HashSet;

pub(crate) fn galgame_portrait_spec_references_lines(
    params_json: &serde_json::Value,
    line_ids: &HashSet<i64>,
) -> Result<bool, String> {
    let source_line_ids = params_json
        .get("source_line_ids")
        .and_then(serde_json::Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "Galgame 立绘差分 source_line_ids 必须是非空数组".to_string())?;
    let mut seen = HashSet::new();
    let mut matches = false;
    for value in source_line_ids {
        let line_id = value
            .as_i64()
            .filter(|line_id| *line_id > 0)
            .ok_or_else(|| "Galgame 立绘差分 source_line_ids 必须是正整数数组".to_string())?;
        if !seen.insert(line_id) {
            return Err("Galgame 立绘差分 source_line_ids 不能包含重复 ID".to_string());
        }
        matches |= line_ids.contains(&line_id);
    }
    Ok(matches)
}

pub async fn galgame_webgal_portraits(
    db: &impl crate::DbExecutor,
    root_project_id: i64,
) -> Result<Vec<CharacterPortraitRecord>, GalgameGenerationRecordError> {
    validate_galgame_generation_id(root_project_id, "root_project_id")?;
    character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(root_project_id))
        .order_by_asc(character_portrait::Column::Id)
        .all(db)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(crate::character::character_portrait_record_from_model)
                .collect()
        })
        .map_err(GalgameGenerationRecordError::Database)
}

pub async fn galgame_portrait_spec_target_profile(
    db: &impl crate::DbExecutor,
    spec: &GenerationSpecRecord,
    input: &GalgamePortraitSpecTargetInput,
) -> Result<GalgamePortraitSpecTargetProfile, GalgameGenerationRecordError> {
    validate_galgame_portrait_spec_target_input(spec, input)?;
    let (project_id, project_kind, root_project_id) =
        novel_project::Entity::find_by_id(spec.project_id)
            .select_only()
            .column(novel_project::Column::Id)
            .column(novel_project::Column::ProjectKind)
            .column(novel_project::Column::RootProjectId)
            .into_tuple::<(i64, String, Option<i64>)>()
            .one(db)
            .await
            .map_err(GalgameGenerationRecordError::Database)?
            .ok_or_else(|| {
                GalgameGenerationRecordError::NotFound(format!("项目不存在: {}", spec.project_id))
            })?;
    if project_kind != ProjectKind::Galgame.as_str() {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分生成意图不属于 Galgame 项目: spec_id={}, project_id={}",
            spec.id, project_id
        )));
    }
    let root_project_id = root_project_id.ok_or_else(|| {
        GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 项目缺少根文本项目: {}",
            project_id
        ))
    })?;
    let source_snapshot_id = validate_galgame_portrait_spec_family_context(spec, root_project_id)?;
    let allowed =
        crate::project::project_source_snapshot_allowed(db, project_id, source_snapshot_id)
            .await
            .map_err(GalgameGenerationRecordError::Database)?;
    if !allowed {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分源快照不属于项目实际使用的来源: spec_id={}, source_snapshot_id={}",
            spec.id, source_snapshot_id
        )));
    }
    validate_galgame_portrait_spec_source_chapter(spec)?;
    let profile = protagonist_profile::Entity::find_by_id(input.profile_id)
        .select_only()
        .column(protagonist_profile::Column::Id)
        .column(protagonist_profile::Column::ProjectId)
        .column(protagonist_profile::Column::Name)
        .into_tuple::<(i64, i64, String)>()
        .one(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .ok_or_else(|| {
            GalgameGenerationRecordError::NotFound(format!(
                "Galgame 立绘差分角色档案不存在: {}",
                input.profile_id
            ))
        })?;
    if profile.1 != root_project_id {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分角色档案不属于当前 Galgame 根项目: spec_id={}, profile_id={}",
            spec.id, profile.0
        )));
    }
    let snapshot_profile = source_snapshot_profile::Entity::find()
        .filter(source_snapshot_profile::Column::SnapshotId.eq(source_snapshot_id))
        .filter(source_snapshot_profile::Column::RootProjectId.eq(root_project_id))
        .filter(source_snapshot_profile::Column::ProfileKind.eq("character"))
        .filter(source_snapshot_profile::Column::SourceProfileId.eq(profile.0))
        .select_only()
        .column(source_snapshot_profile::Column::Id)
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?;
    if snapshot_profile.is_none() {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分角色档案不属于源快照: spec_id={}, profile_id={}, source_snapshot_id={}",
            spec.id, profile.0, source_snapshot_id
        )));
    }
    validate_galgame_portrait_spec_source_lines(
        db,
        spec,
        input,
        project_id,
        root_project_id,
        source_snapshot_id,
    )
    .await?;
    Ok(GalgamePortraitSpecTargetProfile {
        id: profile.0,
        project_id: profile.1,
        name: profile.2,
    })
}

pub(in crate::galgame) fn validate_galgame_portrait_spec_target_input(
    spec: &GenerationSpecRecord,
    input: &GalgamePortraitSpecTargetInput,
) -> Result<(), GalgameGenerationRecordError> {
    validate_galgame_generation_id(spec.id, "generation_spec_id")?;
    validate_galgame_generation_id(spec.project_id, "project_id")?;
    validate_galgame_generation_id(input.profile_id, "profile_id")?;
    require_galgame_generation_text(&input.expression, "expression")?;
    require_galgame_generation_text(&input.pose, "pose")?;
    if let Some(line_id) = input.source_line_ids.iter().find(|line_id| **line_id <= 0) {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "source_line_ids 必须全部是正整数: {line_id}"
        )));
    }
    Ok(())
}

pub async fn completed_galgame_portrait_record(
    db: &impl crate::DbExecutor,
    spec: &GenerationSpecRecord,
    profile_id: i64,
    expression: &str,
    pose: &str,
) -> Result<CharacterPortraitRecord, GalgameGenerationRecordError> {
    validate_galgame_generation_id(spec.id, "generation_spec_id")?;
    validate_galgame_generation_id(spec.root_project_id, "root_project_id")?;
    validate_galgame_generation_id(profile_id, "profile_id")?;
    let expression = require_galgame_generation_text(expression, "expression")?;
    let pose = require_galgame_generation_text(pose, "pose")?;
    character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(spec.root_project_id))
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .filter(character_portrait::Column::IsPrimary.eq(false))
        .filter(character_portrait::Column::Expression.eq(expression))
        .filter(character_portrait::Column::Pose.eq(pose))
        .order_by_desc(character_portrait::Column::Id)
        .one(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .ok_or_else(|| {
            GalgameGenerationRecordError::NotFound(format!(
                "Galgame 立绘差分生成意图已完成但缺少可用立绘: spec_id={}, profile_id={}, expression={}, pose={}",
                spec.id, profile_id, expression, pose
            ))
        })
        .map(crate::character::character_portrait_record_from_model)
}

pub async fn primary_galgame_portrait_reference_image_ids(
    db: &impl crate::DbExecutor,
    profile: &GalgamePortraitSpecTargetProfile,
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
