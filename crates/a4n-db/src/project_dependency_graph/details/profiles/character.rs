use crate::project_dependency_graph::{
    AssetLineageRecord, db_record_result, lineage_ref, optional_prompt_sources, prompt_source,
};
use a4n_core::{error::AppError, models::character::protagonist_profile};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub(in crate::project_dependency_graph) async fn character_profile_detail_records<C>(
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
    let profile = protagonist_profile::Entity::find_by_id(profile_id)
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("人物档案不存在: {profile_id}")))?;
    let text_discovered = profile.is_text_discovered() || profile.last_synced_chapter > 0;
    let mut source_refs = if text_discovered {
        vec![lineage_ref(
            "chapter_analysis",
            "章节分析",
            "text",
            "chapter_analysis",
            None,
        )]
    } else {
        vec![lineage_ref(
            "blueprint",
            "故事蓝图",
            "text",
            "novel_blueprint",
            None,
        )]
    };
    source_refs.push(lineage_ref(
        "character_profiles",
        "人物档案",
        "text",
        "character_profile",
        None,
    ));
    Ok(vec![AssetLineageRecord {
        id: format!("character_profile:{}", profile.id),
        label: profile.name.clone(),
        asset_kind: "character_profile_record".to_string(),
        asset_id: Some(profile.id),
        status: None,
        origin: if text_discovered {
            "llm_generated"
        } else {
            "derived_text"
        }
        .to_string(),
        source_refs,
        prompt_sources: optional_prompt_sources(vec![
            prompt_source(
                "appearance",
                "manual_description",
                profile.appearance.as_deref(),
                Some(profile.id),
                None,
            ),
            prompt_source(
                "personality",
                "manual_description",
                profile.personality.as_deref(),
                Some(profile.id),
                None,
            ),
            prompt_source(
                "background",
                "manual_description",
                profile.background.as_deref(),
                Some(profile.id),
                None,
            ),
        ]),
        media_sources: Vec::new(),
        request_snapshot: None,
        input_status: "missing".to_string(),
        generation_runs: Vec::new(),
        generation_inputs: Vec::new(),
        result: db_record_result("protagonist_profile", &profile),
        warnings: Vec::new(),
    }])
}
