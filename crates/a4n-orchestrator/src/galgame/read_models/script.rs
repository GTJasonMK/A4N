use std::collections::BTreeMap;

use a4n_core::error::AppError;
use a4n_core::models::{
    galgame::{galgame_choice, galgame_line},
    media::{media_asset, media_binding},
};
use a4n_db::galgame::GalgameScriptDetailReadModel;

use crate::galgame::{
    GalgameMediaBindingSummary, GalgameSceneDetail, GalgameScriptDetail, GalgameTextAnchorSummary,
    OrchestratorContext, galgame_speaker_profiles_from_models,
};

impl OrchestratorContext {
    pub async fn get_script_detail(
        &self,
        project_id: i64,
    ) -> Result<GalgameScriptDetail, AppError> {
        let context = self.load_galgame_context(project_id).await?;
        let read_model = a4n_db::galgame::galgame_script_detail_read_model(
            self.db.connection(),
            context.project.id,
        )
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Galgame 剧本不存在: {project_id}")))?;
        galgame_script_detail_from_read_model(read_model)
    }
}

/// 将 DB 层剧本详情 read model 装配为对外 `GalgameScriptDetail`。
pub(in crate::galgame) fn galgame_script_detail_from_read_model(
    read_model: GalgameScriptDetailReadModel,
) -> Result<GalgameScriptDetail, AppError> {
    let GalgameScriptDetailReadModel {
        script,
        speaker_profiles,
        text_anchors,
        media_bindings,
        scenes,
        lines,
        choices,
    } = read_model;
    let mut lines_by_scene = BTreeMap::<i64, Vec<galgame_line::Model>>::new();
    for line in lines {
        lines_by_scene.entry(line.scene_id).or_default().push(line);
    }
    let line_scene = lines_by_scene
        .iter()
        .flat_map(|(scene_id, lines)| lines.iter().map(|line| (line.id, *scene_id)))
        .collect::<BTreeMap<_, _>>();
    let mut choices_by_scene = BTreeMap::<i64, Vec<galgame_choice::Model>>::new();
    for choice in choices {
        if let Some(scene_id) = line_scene.get(&choice.line_id) {
            choices_by_scene.entry(*scene_id).or_default().push(choice);
        }
    }
    let assets = media_bindings
        .assets
        .into_iter()
        .map(|asset| (asset.id, asset))
        .collect::<BTreeMap<_, _>>();
    let media_bindings = galgame_media_binding_summaries(media_bindings.bindings, &assets)?;
    let text_anchors = text_anchors
        .into_iter()
        .map(|anchor| GalgameTextAnchorSummary {
            id: anchor.id,
            source_snapshot_chapter_id: anchor.source_snapshot_chapter_id,
            paragraph_index: anchor.paragraph_index,
            start_offset: anchor.start_offset,
            end_offset: anchor.end_offset,
            quote: anchor.quote,
        })
        .collect();
    Ok(GalgameScriptDetail {
        script,
        speaker_profiles: galgame_speaker_profiles_from_models(&speaker_profiles),
        text_anchors,
        media_bindings,
        scenes: scenes
            .into_iter()
            .map(|scene| GalgameSceneDetail {
                lines: lines_by_scene.remove(&scene.id).unwrap_or_default(),
                choices: choices_by_scene.remove(&scene.id).unwrap_or_default(),
                scene,
            })
            .collect(),
    })
}

pub(in crate::galgame) fn galgame_media_binding_summaries(
    bindings: Vec<media_binding::Model>,
    assets: &BTreeMap<i64, media_asset::Model>,
) -> Result<Vec<GalgameMediaBindingSummary>, AppError> {
    let mut summaries = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let asset = assets
            .get(&binding.media_asset_id)
            .cloned()
            .ok_or_else(|| AppError::InvalidState {
                current: format!(
                    "galgame_media_binding_missing_asset: binding_id={}, media_asset_id={}",
                    binding.id, binding.media_asset_id
                ),
                operation: "load_galgame_media_bindings".to_string(),
            })?;
        summaries.push(GalgameMediaBindingSummary { binding, asset });
    }
    Ok(summaries)
}
