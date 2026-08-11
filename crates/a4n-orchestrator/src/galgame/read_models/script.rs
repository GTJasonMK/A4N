use std::collections::{BTreeMap, BTreeSet};

use a4n_core::error::AppError;
use a4n_core::models::{
    galgame::{galgame_choice, galgame_line, galgame_scene},
    media::{media_asset, media_binding},
};
use a4n_db::galgame::GalgameScriptDetailReadModel;

use crate::galgame::{
    GalgameChoice, GalgameLine, GalgameLineType, GalgameMediaAsset, GalgameMediaBinding,
    GalgameMediaBindingSummary, GalgameMediaRole, GalgameMediaTargetKind, GalgameScene,
    GalgameSceneDetail, GalgameSceneEntryKind, GalgameScriptDetail, GalgameStagePositionV2,
    GalgameTextAnchorSummary, OrchestratorContext, galgame_speaker_profiles_from_models,
    parse_and_validate_presentation_projection, validate_line_semantics,
};

impl OrchestratorContext {
    pub async fn get_script_detail(
        &self,
        project_id: i64,
    ) -> Result<GalgameScriptDetail, AppError> {
        let context = self.load_galgame_context(project_id).await?;
        let read_model = a4n_db::galgame::galgame_script_detail_read_model(
            self.database.connection(),
            context.project.id,
        )
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Galgame 剧本不存在: {project_id}")))?;
        galgame_script_detail_from_read_model(read_model)
    }
}

/// 将 DB 剧本详情严格投影为唯一 Host 合同。
pub(crate) fn galgame_script_detail_from_read_model(
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
    let scene_ids = scenes.iter().map(|scene| scene.id).collect::<BTreeSet<_>>();
    let mut line_scene = BTreeMap::new();
    let mut lines_by_scene = BTreeMap::<i64, Vec<galgame_line::Model>>::new();
    for line in lines {
        if !scene_ids.contains(&line.scene_id) {
            return Err(invalid_read_model(format!(
                "Galgame 行 {} 引用了详情中不存在的场 {}",
                line.id, line.scene_id
            )));
        }
        line_scene.insert(line.id, line.scene_id);
        lines_by_scene.entry(line.scene_id).or_default().push(line);
    }
    let mut choices_by_scene = BTreeMap::<i64, Vec<galgame_choice::Model>>::new();
    for choice in choices {
        let scene_id = line_scene.get(&choice.line_id).copied().ok_or_else(|| {
            invalid_read_model(format!(
                "Galgame choice {} 引用了详情中不存在的行 {}",
                choice.id, choice.line_id
            ))
        })?;
        choices_by_scene.entry(scene_id).or_default().push(choice);
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
    let scenes = scenes
        .into_iter()
        .map(|scene| {
            let scene_id = scene.id;
            Ok(GalgameSceneDetail {
                scene: galgame_scene_from_model(scene)?,
                lines: lines_by_scene
                    .remove(&scene_id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(galgame_line_from_model)
                    .collect::<Result<_, _>>()?,
                choices: choices_by_scene
                    .remove(&scene_id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(galgame_choice_from_model)
                    .collect(),
            })
        })
        .collect::<Result<_, AppError>>()?;
    Ok(GalgameScriptDetail {
        id: script.id,
        root_project_id: script.root_project_id,
        title: script.title,
        speaker_profiles: galgame_speaker_profiles_from_models(&speaker_profiles),
        text_anchors,
        media_bindings,
        scenes,
    })
}

pub(crate) fn galgame_scene_from_model(
    scene: galgame_scene::Model,
) -> Result<GalgameScene, AppError> {
    let entry_kind = GalgameSceneEntryKind::parse(&scene.entry_kind).ok_or_else(|| {
        invalid_read_model(format!(
            "Galgame 场 {} entry_kind 无效: {}",
            scene.id, scene.entry_kind
        ))
    })?;
    Ok(GalgameScene {
        id: scene.id,
        source_snapshot_chapter_id: scene.source_snapshot_chapter_id,
        scene_index: scene.scene_index,
        title: scene.title,
        location: scene.location,
        time_of_day: scene.time_of_day,
        background_note: scene.background_note,
        entry_kind,
        next_scene_id: scene.next_scene_id,
    })
}

pub(crate) fn galgame_line_from_model(line: galgame_line::Model) -> Result<GalgameLine, AppError> {
    let line_type = GalgameLineType::parse(&line.line_type).ok_or_else(|| {
        invalid_read_model(format!(
            "Galgame 行 {} line_type 无效: {}",
            line.id, line.line_type
        ))
    })?;
    let presentation = parse_and_validate_presentation_projection(
        &line.presentation_json,
        line.speaker_profile_id,
        line.expression.as_deref(),
        line.pose.as_deref(),
        line.position.as_deref(),
        &format!("Galgame 行 {}", line.id),
    )
    .map_err(|error| invalid_read_model(error.to_string()))?;
    let expression = line.expression.clone();
    let pose = line.pose.clone();
    let position = line.position.clone();
    validate_line_semantics(
        line_type.as_str(),
        line.speaker_profile_id,
        expression.as_deref(),
        pose.as_deref(),
    )
    .map_err(|error| invalid_read_model(error.to_string()))?;
    let position = position
        .as_deref()
        .map(|value| {
            GalgameStagePositionV2::parse(value).ok_or_else(|| {
                invalid_read_model(format!("Galgame 行 {} position 无效: {value}", line.id))
            })
        })
        .transpose()?;
    Ok(GalgameLine {
        id: line.id,
        scene_id: line.scene_id,
        line_index: line.line_index,
        line_type,
        speaker_profile_id: line.speaker_profile_id,
        text: line.text,
        expression,
        pose,
        position,
        presentation_json: presentation,
        text_anchor_id: line.text_anchor_id,
    })
}

pub(crate) fn galgame_choice_from_model(choice: galgame_choice::Model) -> GalgameChoice {
    GalgameChoice {
        id: choice.id,
        line_id: choice.line_id,
        option_index: choice.option_index,
        option_text: choice.option_text,
        target_scene_id: choice.target_scene_id,
    }
}

pub(crate) fn galgame_media_binding_summary(
    binding: media_binding::Model,
    asset: media_asset::Model,
) -> Result<GalgameMediaBindingSummary, AppError> {
    let target_kind = GalgameMediaTargetKind::parse(&binding.target_kind).ok_or_else(|| {
        invalid_read_model(format!(
            "Galgame 媒体绑定 {} target_kind 无效: {}",
            binding.id, binding.target_kind
        ))
    })?;
    let role = GalgameMediaRole::parse(&binding.role).ok_or_else(|| {
        invalid_read_model(format!(
            "Galgame 媒体绑定 {} role 无效: {}",
            binding.id, binding.role
        ))
    })?;
    Ok(GalgameMediaBindingSummary {
        binding: GalgameMediaBinding {
            id: binding.id,
            media_asset_id: binding.media_asset_id,
            target_kind,
            target_id: binding.target_id,
            role,
        },
        asset: GalgameMediaAsset {
            id: asset.id,
            media_kind: asset.media_kind,
            file_path: asset.file_path,
            thumbnail_path: asset.thumbnail_path,
        },
    })
}

pub(crate) fn galgame_media_binding_summaries(
    bindings: Vec<media_binding::Model>,
    assets: &BTreeMap<i64, media_asset::Model>,
) -> Result<Vec<GalgameMediaBindingSummary>, AppError> {
    bindings
        .into_iter()
        .map(|binding| {
            let asset = assets
                .get(&binding.media_asset_id)
                .cloned()
                .ok_or_else(|| {
                    invalid_read_model(format!(
                        "Galgame 媒体绑定 {} 引用了不存在的资产 {}",
                        binding.id, binding.media_asset_id
                    ))
                })?;
            galgame_media_binding_summary(binding, asset)
        })
        .collect()
}

fn invalid_read_model(operation: impl Into<String>) -> AppError {
    AppError::InvalidState {
        current: "galgame_read_model".to_string(),
        operation: operation.into(),
    }
}
