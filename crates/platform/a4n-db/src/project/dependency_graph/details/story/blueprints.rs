use crate::project::dependency_graph::AssetLineageRecord;
use crate::project::dependency_graph::details::records::{
    db_record_result, lineage_ref, optional_prompt_sources, prompt_source,
};
use a4n_db_models::models::project::{blueprint_version, novel_blueprint};
use a4n_error::AppError;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(in crate::project::dependency_graph) async fn blueprint_detail_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let blueprints = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .order_by_asc(novel_blueprint::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(blueprints
        .into_iter()
        .map(|blueprint| AssetLineageRecord {
            id: format!("novel_blueprint:{}", blueprint.id),
            label: blueprint.title.clone(),
            asset_kind: "novel_blueprint".to_string(),
            asset_id: Some(blueprint.id),
            status: Some(blueprint.current_version.clone()),
            origin: "llm_generated".to_string(),
            source_refs: vec![lineage_ref(
                "inspiration",
                "灵感对话",
                "text",
                "inspiration_conversation",
                None,
            )],
            prompt_sources: optional_prompt_sources(vec![prompt_source(
                &format!("novel_blueprint_synopsis:{}", blueprint.id),
                "blueprint_synopsis",
                blueprint.full_synopsis.as_deref(),
                Some(blueprint.id),
                None,
            )]),
            media_sources: Vec::new(),
            request_snapshot: None,
            input_status: "missing".to_string(),
            generation_inputs: Vec::new(),
            result: db_record_result("novel_blueprint", &blueprint),
            warnings: Vec::new(),
        })
        .collect())
}

pub(in crate::project::dependency_graph) async fn blueprint_version_detail_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let versions = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_version::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(versions
        .into_iter()
        .map(|version| AssetLineageRecord {
            id: format!("blueprint_version:{}", version.id),
            label: format!("蓝图版本 {}", version.version),
            asset_kind: "blueprint_version".to_string(),
            asset_id: Some(version.id),
            status: Some(version.version.clone()),
            origin: "derived_text".to_string(),
            source_refs: vec![lineage_ref(
                "blueprint",
                "故事蓝图",
                "text",
                "novel_blueprint",
                None,
            )],
            prompt_sources: Vec::new(),
            media_sources: Vec::new(),
            request_snapshot: None,
            input_status: "missing".to_string(),
            generation_inputs: Vec::new(),
            result: db_record_result("blueprint_version", &version),
            warnings: Vec::new(),
        })
        .collect())
}
