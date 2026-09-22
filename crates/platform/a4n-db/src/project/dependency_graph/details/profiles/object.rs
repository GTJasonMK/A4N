use crate::project::dependency_graph::AssetLineageRecord;
use crate::project::dependency_graph::details::records::{
    db_record_result, lineage_ref, optional_prompt_sources, prompt_source,
};
use a4n_db_models::models::asset::object_profile;
use a4n_error::AppError;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub(in crate::project::dependency_graph) async fn object_profile_detail_records<C>(
    db: &C,
    project_id: i64,
    record_id: Option<i64>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let Some(profile_id) = record_id else {
        return Ok(Vec::new());
    };
    let profile = object_profile::Entity::find_by_id(profile_id)
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("物体档案不存在: {profile_id}")))?;
    Ok(vec![AssetLineageRecord {
        id: format!("object_profile:{}", profile.id),
        label: profile.name.clone(),
        asset_kind: "object_profile_record".to_string(),
        asset_id: Some(profile.id),
        status: None,
        origin: "llm_generated".to_string(),
        source_refs: vec![
            lineage_ref(
                "chapter_analysis",
                "章节分析",
                "text",
                "chapter_analysis",
                None,
            ),
            lineage_ref(
                "object_profiles",
                "物体档案",
                "text",
                "object_profile",
                None,
            ),
        ],
        prompt_sources: optional_prompt_sources(vec![
            prompt_source(
                "visual_description",
                "manual_description",
                Some(&profile.visual_description),
                Some(profile.id),
                None,
            ),
            prompt_source(
                "image_prompt_seed",
                "manual_description",
                Some(&profile.image_prompt_seed),
                Some(profile.id),
                None,
            ),
        ]),
        media_sources: Vec::new(),
        request_snapshot: None,
        input_status: "missing".to_string(),
        generation_inputs: Vec::new(),
        result: db_record_result("object_profile", &profile),
        warnings: Vec::new(),
    }])
}
