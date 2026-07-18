use crate::project_dependency_graph::{
    AssetLineageRecord, db_record_result, lineage_ref, optional_prompt_sources, prompt_source,
};
use a4n_core::{error::AppError, models::scene::scene_profile};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub(in crate::project_dependency_graph) async fn scene_profile_detail_records<C>(
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
    let profile = scene_profile::Entity::find_by_id(profile_id)
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("场景档案不存在: {profile_id}")))?;
    Ok(vec![AssetLineageRecord {
        id: format!("scene_profile:{}", profile.id),
        label: profile.name.clone(),
        asset_kind: "scene_profile_record".to_string(),
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
            lineage_ref("scene_profiles", "场景档案", "text", "scene_profile", None),
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
        generation_runs: Vec::new(),
        generation_inputs: Vec::new(),
        result: db_record_result("scene_profile", &profile),
        warnings: Vec::new(),
    }])
}
