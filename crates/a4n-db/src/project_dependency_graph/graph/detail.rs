use a4n_core::{error::AppError, models::project::novel_project};
use sea_orm::{ConnectionTrait, EntityTrait};

use super::asset_dependency_graph;
use crate::project_dependency_graph::{
    AssetGraphNodeDetail, CHAPTER_VIDEO_PROMPT_TYPES, VIDEO_FRAME_PROMPT_TYPES,
    VIDEO_SEGMENT_PLAN_PROMPT_TYPES,
    details::{
        blueprint_detail_records, blueprint_version_detail_records,
        chapter_analysis_detail_records, chapter_content_detail_records,
        chapter_outline_detail_records, character_portrait_detail_records,
        character_profile_detail_records, enrich_lineage_records_with_generation_inputs,
        generated_video_detail_records, generation_spec_detail_records,
        generic_node_detail_records, manga_image_detail_records, manga_prompt_detail_records,
        media_asset_detail_records, object_profile_detail_records, part_outline_detail_records,
        profile_material_image_detail_records, prompt_artifact_detail_records,
        scene_profile_detail_records, structural_source_refs_for_node,
        video_frame_image_detail_records, video_prompt_detail_records,
    },
    scope::load_asset_graph_scope,
};

pub async fn asset_dependency_graph_node_detail(
    db: &impl ConnectionTrait,
    project_id: i64,
    node_id: &str,
) -> Result<AssetGraphNodeDetail, AppError> {
    let requested_project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let scope = load_asset_graph_scope(db, requested_project).await?;
    let root_project_id = scope.root_project.id;
    let family_project_ids = scope.family_project_ids;
    let graph = asset_dependency_graph(db, project_id).await?;
    let node = graph
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("依赖图节点不存在: {node_id}")))?;
    let structural_source_refs = structural_source_refs_for_node(&graph, &node.id);
    let mut warnings = Vec::new();
    let mut records = match node.asset_type.as_str() {
        "novel_blueprint" => blueprint_detail_records(db, root_project_id).await?,
        "blueprint_version" => blueprint_version_detail_records(db, root_project_id).await?,
        "part_outline" => part_outline_detail_records(db, root_project_id).await?,
        "chapter_outline" => chapter_outline_detail_records(db, root_project_id).await?,
        "chapter_content" => chapter_content_detail_records(db, root_project_id).await?,
        "chapter_analysis" => chapter_analysis_detail_records(db, root_project_id).await?,
        "character_profile_record" => {
            character_profile_detail_records(db, root_project_id, node.record_id).await?
        }
        "object_profile_record" => {
            object_profile_detail_records(db, root_project_id, node.record_id).await?
        }
        "scene_profile_record" => {
            scene_profile_detail_records(db, root_project_id, node.record_id).await?
        }
        "video_segment_plan" => {
            video_prompt_detail_records(db, &family_project_ids, VIDEO_SEGMENT_PLAN_PROMPT_TYPES)
                .await?
        }
        "character_portrait" => {
            character_portrait_detail_records(db, &family_project_ids, node.profile_id).await?
        }
        "object_material_image" => {
            profile_material_image_detail_records(
                db,
                &family_project_ids,
                node.profile_id,
                crate::profile_material::PROFILE_KIND_OBJECT,
            )
            .await?
        }
        "scene_material_image" => {
            profile_material_image_detail_records(
                db,
                &family_project_ids,
                node.profile_id,
                crate::profile_material::PROFILE_KIND_SCENE,
            )
            .await?
        }
        "chapter_manga_prompt" => manga_prompt_detail_records(db, &family_project_ids).await?,
        "manga_page_image" => {
            manga_image_detail_records(db, &family_project_ids, &mut warnings).await?
        }
        "video_frame_prompt" => {
            video_prompt_detail_records(db, &family_project_ids, VIDEO_FRAME_PROMPT_TYPES).await?
        }
        "video_frame_image" => {
            video_frame_image_detail_records(db, &family_project_ids, &mut warnings).await?
        }
        "chapter_video_prompt" => {
            video_prompt_detail_records(db, &family_project_ids, CHAPTER_VIDEO_PROMPT_TYPES).await?
        }
        "generated_video" => {
            generated_video_detail_records(db, &family_project_ids, &mut warnings).await?
        }
        "generation_spec" => generation_spec_detail_records(db, &family_project_ids).await?,
        "prompt_artifact" => prompt_artifact_detail_records(db, &family_project_ids).await?,
        "media_asset" => media_asset_detail_records(db, &family_project_ids, &mut warnings).await?,
        _ => generic_node_detail_records(&node, structural_source_refs.clone()),
    };
    for record in &mut records {
        if record.source_refs.is_empty() {
            record.source_refs = structural_source_refs.clone();
        }
    }
    enrich_lineage_records_with_generation_inputs(db, &mut records).await?;

    Ok(AssetGraphNodeDetail {
        node_id: node.id,
        title: node.label,
        kind: node.kind,
        asset_type: node.asset_type,
        summary: node.summary,
        records,
        warnings,
    })
}
