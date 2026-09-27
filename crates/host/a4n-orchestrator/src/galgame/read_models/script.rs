use std::collections::{HashMap, HashSet};

use a4n_db::galgame::{
    GalgameMediaAssetReadRecord, GalgameMediaBindingReadRecord, GalgameScriptDetailChoiceRecord,
    GalgameScriptDetailReadModel, GalgameScriptDetailSceneRecord,
};
use a4n_db_models::models::{
    galgame::{galgame_choice, galgame_line, galgame_scene},
    media::{media_asset, media_binding},
};
use a4n_error::AppError;

use crate::galgame::{
    GalgameChoice, GalgameLine, GalgameLineType, GalgameMediaAsset, GalgameMediaBinding,
    GalgameMediaBindingSummary, GalgameMediaRole, GalgameMediaTargetKind, GalgameScene,
    GalgameSceneDetail, GalgameSceneEntryKind, GalgameScriptDetail, GalgameStagePositionV2,
    GalgameTextAnchorSummary, OrchestratorContext, galgame_speaker_profiles_from_prompt_records,
    parse_and_validate_presentation_projection, validate_line_semantics,
};

impl OrchestratorContext {
    pub async fn get_script_detail(
        &self,
        project_id: i64,
    ) -> Result<GalgameScriptDetail, AppError> {
        self.find_script_detail(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Galgame 剧本不存在: {project_id}")))
    }

    pub(crate) async fn find_script_detail(
        &self,
        project_id: i64,
    ) -> Result<Option<GalgameScriptDetail>, AppError> {
        let context = self.load_galgame_context(project_id).await?;
        let read_model = a4n_db::galgame::galgame_script_detail_read_model(
            self.database.connection(),
            context.project.id,
        )
        .await
        .map_err(AppError::Database)?;
        read_model
            .map(galgame_script_detail_from_read_model)
            .transpose()
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
    let scene_ids = scenes.iter().map(|scene| scene.id).collect::<HashSet<_>>();
    let mut line_scene = HashMap::new();
    let mut lines_by_scene = HashMap::<i64, Vec<galgame_line::Model>>::new();
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
    let mut choices_by_scene = HashMap::<i64, Vec<GalgameScriptDetailChoiceRecord>>::new();
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
        .collect::<HashMap<_, _>>();
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
                scene: galgame_scene_from_detail_record(scene)?,
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
                    .map(galgame_choice_from_detail_record)
                    .collect(),
            })
        })
        .collect::<Result<_, AppError>>()?;
    Ok(GalgameScriptDetail {
        id: script.id,
        root_project_id: script.root_project_id,
        title: script.title,
        speaker_profiles: galgame_speaker_profiles_from_prompt_records(&speaker_profiles),
        text_anchors,
        media_bindings,
        scenes,
    })
}

pub(crate) fn galgame_scene_from_model(
    scene: galgame_scene::Model,
) -> Result<GalgameScene, AppError> {
    galgame_scene_from_parts(
        scene.id,
        scene.source_snapshot_chapter_id,
        scene.scene_index,
        scene.title,
        scene.location,
        scene.time_of_day,
        scene.background_note,
        scene.entry_kind,
        scene.next_scene_id,
    )
}

fn galgame_scene_from_detail_record(
    scene: GalgameScriptDetailSceneRecord,
) -> Result<GalgameScene, AppError> {
    galgame_scene_from_parts(
        scene.id,
        scene.source_snapshot_chapter_id,
        scene.scene_index,
        scene.title,
        scene.location,
        scene.time_of_day,
        scene.background_note,
        scene.entry_kind,
        scene.next_scene_id,
    )
}

fn galgame_scene_from_parts(
    id: i64,
    source_snapshot_chapter_id: i64,
    scene_index: i32,
    title: String,
    location: Option<String>,
    time_of_day: Option<String>,
    background_note: Option<String>,
    entry_kind_value: String,
    next_scene_id: Option<i64>,
) -> Result<GalgameScene, AppError> {
    let entry_kind = GalgameSceneEntryKind::parse(&entry_kind_value).ok_or_else(|| {
        invalid_read_model(format!(
            "Galgame 场 {} entry_kind 无效: {}",
            id, entry_kind_value
        ))
    })?;
    Ok(GalgameScene {
        id,
        source_snapshot_chapter_id,
        scene_index,
        title,
        location,
        time_of_day,
        background_note,
        entry_kind,
        next_scene_id,
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
    .map_err(|error| invalid_read_model(format!("Galgame 行 {}: {error}", line.id)))?;
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
    galgame_choice_from_parts(
        choice.id,
        choice.line_id,
        choice.option_index,
        choice.option_text,
        choice.target_scene_id,
    )
}

fn galgame_choice_from_detail_record(choice: GalgameScriptDetailChoiceRecord) -> GalgameChoice {
    galgame_choice_from_parts(
        choice.id,
        choice.line_id,
        choice.option_index,
        choice.option_text,
        choice.target_scene_id,
    )
}

fn galgame_choice_from_parts(
    id: i64,
    line_id: i64,
    option_index: i32,
    option_text: String,
    target_scene_id: Option<i64>,
) -> GalgameChoice {
    GalgameChoice {
        id,
        line_id,
        option_index,
        option_text,
        target_scene_id,
    }
}

pub(crate) fn galgame_media_binding_summary(
    binding: media_binding::Model,
    asset: media_asset::Model,
) -> Result<GalgameMediaBindingSummary, AppError> {
    galgame_media_binding_summary_parts(
        binding.id,
        binding.media_asset_id,
        binding.target_kind,
        binding.target_id,
        binding.role,
        asset.id,
        asset.media_kind,
        asset.file_path,
        asset.thumbnail_path,
    )
}

pub(crate) fn galgame_media_binding_summaries(
    bindings: Vec<GalgameMediaBindingReadRecord>,
    assets: &HashMap<i64, GalgameMediaAssetReadRecord>,
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
            galgame_media_binding_summary_parts(
                binding.id,
                binding.media_asset_id,
                binding.target_kind,
                binding.target_id,
                binding.role,
                asset.id,
                asset.media_kind,
                asset.file_path,
                asset.thumbnail_path,
            )
        })
        .collect()
}

fn galgame_media_binding_summary_parts(
    binding_id: i64,
    media_asset_id: i64,
    target_kind_value: String,
    target_id: i64,
    role_value: String,
    asset_id: i64,
    media_kind: String,
    file_path: Option<String>,
    thumbnail_path: Option<String>,
) -> Result<GalgameMediaBindingSummary, AppError> {
    let target_kind = GalgameMediaTargetKind::parse(&target_kind_value).ok_or_else(|| {
        invalid_read_model(format!(
            "Galgame 媒体绑定 {} target_kind 无效: {}",
            binding_id, target_kind_value
        ))
    })?;
    let role = GalgameMediaRole::parse(&role_value).ok_or_else(|| {
        invalid_read_model(format!(
            "Galgame 媒体绑定 {} role 无效: {}",
            binding_id, role_value
        ))
    })?;
    Ok(GalgameMediaBindingSummary {
        binding: GalgameMediaBinding {
            id: binding_id,
            media_asset_id,
            target_kind,
            target_id,
            role,
        },
        asset: GalgameMediaAsset {
            id: asset_id,
            media_kind,
            file_path,
            thumbnail_path,
        },
    })
}

fn invalid_read_model(operation: impl Into<String>) -> AppError {
    AppError::InvalidState {
        current: "galgame_read_model".to_string(),
        operation: operation.into(),
    }
}
