use std::collections::{BTreeMap, BTreeSet};

use a4n_api_contract::export::ExportFileData;
use a4n_archive_core::{ArchiveEntry, write_zip};
use a4n_domain::{
    character::CharacterPortraitRecord,
    galgame::{GalgameFacingV2, GalgameStageLayerV2},
};
use a4n_error::AppError;
use base64::Engine as _;

use crate::OrchestratorContext;

use super::{
    GalgameChoice, GalgameLineType, GalgameMediaRole, GalgameMediaTargetKind, GalgameSceneDetail,
    GalgameSceneEntryKind, GalgameScriptDetail,
    webgal_export_rules::{
        media_file_extension, media_file_extension_for_webgal_role, sanitize_export_file_stem,
        webgal_choice_text, webgal_string_literal, webgal_transition_args,
    },
};
use crate::media::galgame_media_binding_required_media_kind;
use std::path::Path;

#[derive(Debug, Clone)]
struct WebgalExportAsset {
    script_name: String,
    zip_path: String,
    data_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct WebgalFigureKey {
    profile_id: i64,
    expression: Option<String>,
    pose: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebgalFigureState {
    script_name: String,
    position: String,
    facing: GalgameFacingV2,
    layer: GalgameStageLayerV2,
}

#[derive(Default)]
struct WebgalExportAssets {
    scene_backgrounds: BTreeMap<i64, WebgalExportAsset>,
    line_cgs: BTreeMap<i64, WebgalExportAsset>,
    line_bgms: BTreeMap<i64, WebgalExportAsset>,
    line_sfx: BTreeMap<i64, WebgalExportAsset>,
    figures: BTreeMap<WebgalFigureKey, WebgalExportAsset>,
}

impl OrchestratorContext {
    pub async fn run_export_galgame_webgal_validated(
        &self,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        let detail = self.get_script_detail(project_id).await?;
        if detail.scenes.is_empty() {
            return Err(AppError::InvalidState {
                current: "empty_galgame_script".to_string(),
                operation: "导出 WebGAL".to_string(),
            });
        }
        let portraits = self
            .galgame_webgal_portraits(detail.root_project_id)
            .await?;
        let bytes = build_webgal_package(self.data_dir(), &detail, &portraits)?;
        let file_stem = sanitize_export_file_stem(&detail.title).map_err(invalid_webgal)?;
        Ok(ExportFileData {
            file_name: format!("{file_stem}-webgal.zip"),
            mime_type: "application/zip".to_string(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        })
    }
}

fn build_webgal_package(
    data_dir: &Path,
    detail: &GalgameScriptDetail,
    portraits: &[CharacterPortraitRecord],
) -> Result<Vec<u8>, AppError> {
    let scene_file_by_id = detail
        .scenes
        .iter()
        .enumerate()
        .map(|(index, scene)| {
            (
                scene.scene.id,
                format!("scene_{:03}_{}.txt", index + 1, scene.scene.id),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let scene_entry_by_id = detail
        .scenes
        .iter()
        .map(|scene| (scene.scene.id, scene.scene.entry_kind))
        .collect::<BTreeMap<_, _>>();
    let speaker_name_by_id = detail
        .speaker_profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<BTreeMap<_, _>>();
    let choices_by_line = detail
        .scenes
        .iter()
        .flat_map(|scene| scene.choices.iter())
        .fold(BTreeMap::<i64, Vec<_>>::new(), |mut choices, choice| {
            choices.entry(choice.line_id).or_default().push(choice);
            choices
        });
    let figure_profile_ids = detail
        .scenes
        .iter()
        .flat_map(|scene| &scene.lines)
        .flat_map(|line| &line.presentation_json.visible_cast)
        .map(|member| member.character_profile_id)
        .collect::<BTreeSet<_>>();
    validate_webgal_choice_contract(detail, &scene_file_by_id)?;
    let assets = webgal_export_assets(detail, portraits)?;
    let first_scene = scene_file_by_id
        .get(&detail.scenes[0].scene.id)
        .ok_or_else(|| invalid_webgal("Galgame 首场导出文件名缺失"))?;

    let mut entries = vec![ArchiveEntry {
        path: "game/scene/start.txt".to_string(),
        bytes: format!("changeScene:{first_scene};\n").into_bytes(),
        label: "WebGAL 启动场景".to_string(),
    }];
    for scene in &detail.scenes {
        let file_name = scene_file_by_id.get(&scene.scene.id).ok_or_else(|| {
            invalid_webgal(format!("Galgame 场导出文件名缺失: {}", scene.scene.id))
        })?;
        let script = render_webgal_scene(
            scene,
            &scene_file_by_id,
            &scene_entry_by_id,
            &speaker_name_by_id,
            &choices_by_line,
            &assets,
            &figure_profile_ids,
        )?;
        entries.push(ArchiveEntry {
            path: format!("game/scene/{file_name}"),
            bytes: script.into_bytes(),
            label: format!("WebGAL 场景 {}", scene.scene.id),
        });
    }

    let mut written_assets = BTreeSet::new();
    for asset in assets
        .scene_backgrounds
        .values()
        .chain(assets.line_cgs.values())
        .chain(assets.line_bgms.values())
        .chain(assets.line_sfx.values())
        .chain(assets.figures.values())
    {
        if written_assets.insert(asset.zip_path.clone()) {
            entries.push(ArchiveEntry {
                path: asset.zip_path.clone(),
                bytes: read_webgal_data_file(data_dir, &asset.data_path)?,
                label: format!("WebGAL 媒体 {}", asset.zip_path),
            });
        }
    }

    let config = webgal_config_text(detail)?;
    write_zip(
        std::iter::once(ArchiveEntry {
            path: "game/config.txt".to_string(),
            bytes: config.into_bytes(),
            label: "WebGAL 配置".to_string(),
        })
        .chain(entries),
        "WebGAL zip 包",
    )
    .map_err(|error| invalid_webgal(error.to_string()))
}

fn render_webgal_scene(
    scene: &GalgameSceneDetail,
    scene_file_by_id: &BTreeMap<i64, String>,
    scene_entry_by_id: &BTreeMap<i64, GalgameSceneEntryKind>,
    speaker_name_by_id: &BTreeMap<i64, String>,
    choices_by_line: &BTreeMap<i64, Vec<&GalgameChoice>>,
    assets: &WebgalExportAssets,
    figure_profile_ids: &BTreeSet<i64>,
) -> Result<String, AppError> {
    let mut output = String::new();
    let mut current_figure_by_profile = BTreeMap::<i64, WebgalFigureState>::new();
    for profile_id in figure_profile_ids {
        output.push_str(&format!(
            "changeFigure:none -id=a4n_profile_{profile_id} -next;\n"
        ));
    }
    if let Some(background) = assets.scene_backgrounds.get(&scene.scene.id) {
        output.push_str(&format!("changeBg:{} -next;\n", background.script_name));
    }
    for line in &scene.lines {
        if let Some(bgm) = assets.line_bgms.get(&line.id) {
            output.push_str(&format!("bgm:{};\n", bgm.script_name));
        }
        if let Some(sfx) = assets.line_sfx.get(&line.id) {
            output.push_str(&format!("playEffect:{} -next;\n", sfx.script_name));
        }
        if let Some(cg) = assets.line_cgs.get(&line.id) {
            let transition_args = webgal_transition_args(line.presentation_json.transition)
                .map_err(invalid_webgal)?;
            let name_var = format!("__a4n_cg_name_{}", line.id);
            output.push_str(&format!(
                "setVar:{name_var}=({});\n",
                webgal_string_literal(&format!("{} #{} CG", scene.scene.title, line.line_index))
            ));
            output.push_str(&format!(
                "changeBg:{}{} -next;\n",
                cg.script_name, transition_args
            ));
            output.push_str(&format!(
                "unlockCg:{} -name={{{name_var}}};\n",
                cg.script_name
            ));
        }
        let visible_figures = line
            .presentation_json
            .visible_cast
            .iter()
            .filter_map(|member| {
                webgal_cast_figure(member, assets)
                    .map(|figure| (member.character_profile_id, (member, figure)))
            })
            .collect::<BTreeMap<_, _>>();
        for profile_id in current_figure_by_profile.keys() {
            if visible_figures.contains_key(profile_id) {
                continue;
            }
            if line.presentation_json.exit.contains(profile_id) {
                output.push_str(&format!(
                    "setTransition: -target=a4n_profile_{profile_id} -exit=exit;\n"
                ));
            }
            output.push_str(&format!(
                "changeFigure:none -id=a4n_profile_{profile_id} -next;\n"
            ));
        }
        current_figure_by_profile.retain(|profile_id, _| visible_figures.contains_key(profile_id));
        for (profile_id, (member, figure)) in visible_figures {
            let state = WebgalFigureState {
                script_name: figure.script_name.clone(),
                position: member.position.to_string(),
                facing: member.facing,
                layer: member.layer,
            };
            if current_figure_by_profile.get(&profile_id) == Some(&state) {
                continue;
            }
            let layer = match member.layer {
                GalgameStageLayerV2::Back => 1,
                GalgameStageLayerV2::Middle => 2,
                GalgameStageLayerV2::Front => 3,
            };
            let facing_scale = if member.facing == GalgameFacingV2::Left {
                -1
            } else {
                1
            };
            let enter = if line.presentation_json.enter.contains(&profile_id) {
                match member.position.as_str() {
                    "left" => " -enter=enter-from-left",
                    "right" => " -enter=enter-from-right",
                    _ => " -enter=enter",
                }
            } else {
                ""
            };
            output.push_str(&format!(
                "changeFigure:{} -id=a4n_profile_{profile_id} -{} -zIndex={layer} -transform={{\"scale\":{{\"x\":{facing_scale},\"y\":1}}}}{} -next;\n",
                figure.script_name,
                state.position,
                enter,
            ));
            current_figure_by_profile.insert(profile_id, state);
        }
        output.push_str(&format!(
            "setVar:__a4n_line_text=({});\n",
            webgal_string_literal(&line.text)
        ));
        match line.line_type {
            GalgameLineType::Dialogue | GalgameLineType::Monologue => {
                let speaker_profile_id = line.speaker_profile_id.ok_or_else(|| {
                    invalid_webgal(format!(
                        "Galgame {} 行 {} 缺少 speaker_profile_id，无法导出 WebGAL",
                        line.line_type, line.id
                    ))
                })?;
                let speaker = speaker_name_by_id.get(&speaker_profile_id).ok_or_else(|| {
                    invalid_webgal(format!(
                        "Galgame {} 行 {} 绑定的角色档案 {} 不存在，无法导出 WebGAL",
                        line.line_type, line.id, speaker_profile_id
                    ))
                })?;
                output.push_str(&format!(
                    "setVar:__a4n_line_speaker=({});\nsay:{{__a4n_line_text}} -speaker={{__a4n_line_speaker}};\n",
                    webgal_string_literal(speaker)
                ));
            }
            GalgameLineType::Narration | GalgameLineType::StageDirection => {
                output.push_str("say:{__a4n_line_text} -clear;\n");
            }
            GalgameLineType::Choice => {
                let choices = choices_by_line
                    .get(&line.id)
                    .filter(|choices| !choices.is_empty())
                    .ok_or_else(|| {
                        invalid_webgal(format!(
                            "Galgame choice 行 {} 缺少 galgame_choice 选项，无法导出 WebGAL",
                            line.id
                        ))
                    })?;
                let mut sorted_choices = choices.clone();
                sorted_choices.sort_by_key(|choice| choice.option_index);
                let options = sorted_choices
                    .iter()
                    .map(|choice| {
                        let target_scene_id = choice.target_scene_id.ok_or_else(|| {
                            invalid_webgal(format!(
                                "Galgame choice {} 缺少 target_scene_id，无法导出 WebGAL",
                                choice.id
                            ))
                        })?;
                        let target = scene_file_by_id.get(&target_scene_id).ok_or_else(|| {
                            invalid_webgal(format!(
                                "Galgame choice {} 指向不存在的场 {}，无法导出 WebGAL",
                                choice.id, target_scene_id
                            ))
                        })?;
                        let text_var = format!("__a4n_choice_{}_{}", line.id, choice.option_index);
                        let choice_text =
                            webgal_choice_text(&choice.option_text).map_err(invalid_webgal)?;
                        output.push_str(&format!(
                            "setVar:{text_var}=({});\n",
                            webgal_string_literal(&choice_text)
                        ));
                        Ok(format!("{{{text_var}}}:{target}"))
                    })
                    .collect::<Result<Vec<_>, AppError>>()?
                    .join("|");
                output.push_str(&format!(
                    "say:{{__a4n_line_text}} -clear -next;\nchoose:{options};\n"
                ));
            }
        }
    }
    match scene.scene.next_scene_id {
        Some(next_scene_id) => {
            let next_file = scene_file_by_id.get(&next_scene_id).ok_or_else(|| {
                invalid_webgal(format!(
                    "Galgame 场 {} next_scene_id 指向不存在的场 {next_scene_id}，无法导出 WebGAL",
                    scene.scene.id
                ))
            })?;
            if scene.scene.entry_kind == GalgameSceneEntryKind::Branch
                && scene_entry_by_id.get(&next_scene_id) != Some(&GalgameSceneEntryKind::Main)
            {
                return Err(invalid_webgal(format!(
                    "Galgame branch 场 {} next_scene_id 必须指向主线场，当前指向 {next_scene_id}",
                    scene.scene.id
                )));
            }
            output.push_str(&format!("changeScene:{next_file};\n"));
        }
        None if scene.scene.entry_kind == GalgameSceneEntryKind::Branch => {
            return Err(invalid_webgal(format!(
                "Galgame branch 场 {} 缺少 next_scene_id，无法导出 WebGAL",
                scene.scene.id
            )));
        }
        None => {}
    }
    Ok(output)
}

fn webgal_cast_figure<'a>(
    member: &a4n_domain::galgame::GalgameVisibleCastMemberV2,
    assets: &'a WebgalExportAssets,
) -> Option<&'a WebgalExportAsset> {
    let key = WebgalFigureKey {
        profile_id: member.character_profile_id,
        expression: Some(member.expression.clone()),
        pose: Some(member.pose.clone()),
    };
    assets.figures.get(&key)
}

fn webgal_export_assets(
    detail: &GalgameScriptDetail,
    portraits: &[CharacterPortraitRecord],
) -> Result<WebgalExportAssets, AppError> {
    let mut assets = WebgalExportAssets::default();
    for item in &detail.media_bindings {
        let binding = &item.binding;
        let asset = &item.asset;
        let expected_media_kind = galgame_media_binding_required_media_kind(
            binding.target_kind.as_str(),
            binding.role.as_str(),
        )
        .ok_or_else(|| {
            invalid_webgal(format!(
                "Galgame 媒体绑定目标/角色无效，无法导出 WebGAL: binding_id={}, target_kind={}, role={}",
                binding.id, binding.target_kind, binding.role
            ))
        })?;
        if asset.media_kind != expected_media_kind {
            return Err(invalid_webgal(format!(
                "Galgame 媒体绑定资产类型不匹配，无法导出 WebGAL: binding_id={}, media_asset_id={}, role={}, expected_media_kind={}, actual_media_kind={}",
                binding.id, asset.id, binding.role, expected_media_kind, asset.media_kind
            )));
        }
        let file_path = asset
            .file_path
            .as_deref()
            .ok_or_else(|| invalid_webgal(format!("媒体资产 {} 缺少文件路径", asset.id)))?;
        let extension = media_file_extension_for_webgal_role(file_path, binding.role)
            .map_err(invalid_webgal)?;
        let exported = |directory: &str, prefix: &str| WebgalExportAsset {
            script_name: format!("{prefix}_{}_{}.{}", binding.target_id, asset.id, extension),
            zip_path: format!(
                "game/{directory}/{prefix}_{}_{}.{}",
                binding.target_id, asset.id, extension
            ),
            data_path: file_path.to_string(),
        };
        match (binding.target_kind, binding.role) {
            (GalgameMediaTargetKind::GalgameScene, GalgameMediaRole::Background) => {
                assets
                    .scene_backgrounds
                    .insert(binding.target_id, exported("background", "background"));
            }
            (GalgameMediaTargetKind::GalgameLine, GalgameMediaRole::Cg) => {
                assets
                    .line_cgs
                    .insert(binding.target_id, exported("background", "cg"));
            }
            (GalgameMediaTargetKind::GalgameLine, GalgameMediaRole::Bgm) => {
                assets
                    .line_bgms
                    .insert(binding.target_id, exported("bgm", "bgm"));
            }
            (GalgameMediaTargetKind::GalgameLine, GalgameMediaRole::Sfx) => {
                assets
                    .line_sfx
                    .insert(binding.target_id, exported("vocal", "sfx"));
            }
            _ => {
                return Err(invalid_webgal(format!(
                    "Galgame 媒体绑定目标/角色无效，无法导出 WebGAL: binding_id={}, target_kind={}, role={}",
                    binding.id, binding.target_kind, binding.role
                )));
            }
        }
    }
    for portrait in portraits {
        let extension = media_file_extension(&portrait.image_path).map_err(invalid_webgal)?;
        let key = if portrait.is_primary && portrait.expression.is_none() && portrait.pose.is_none()
        {
            WebgalFigureKey {
                profile_id: portrait.profile_id,
                expression: None,
                pose: None,
            }
        } else {
            WebgalFigureKey {
                profile_id: portrait.profile_id,
                expression: portrait.expression.clone(),
                pose: portrait.pose.clone(),
            }
        };
        assets.figures.insert(
            key,
            WebgalExportAsset {
                script_name: format!(
                    "figure_{}_{}.{}",
                    portrait.profile_id, portrait.id, extension
                ),
                zip_path: format!(
                    "game/figure/figure_{}_{}.{}",
                    portrait.profile_id, portrait.id, extension
                ),
                data_path: portrait.image_path.clone(),
            },
        );
    }
    Ok(assets)
}

fn validate_webgal_choice_contract(
    detail: &GalgameScriptDetail,
    scene_file_by_id: &BTreeMap<i64, String>,
) -> Result<(), AppError> {
    for scene in &detail.scenes {
        let line_type_by_id = scene
            .lines
            .iter()
            .map(|line| (line.id, line.line_type))
            .collect::<BTreeMap<_, _>>();
        let mut choices_by_line = BTreeMap::<i64, Vec<&GalgameChoice>>::new();
        for choice in &scene.choices {
            match line_type_by_id.get(&choice.line_id).copied() {
                Some(GalgameLineType::Choice) => {}
                Some(line_type) => {
                    return Err(invalid_webgal(format!(
                        "Galgame choice {} 挂在非 choice 行下，无法导出 WebGAL: line_id={}, line_type={line_type}",
                        choice.id, choice.line_id
                    )));
                }
                None => {
                    return Err(invalid_webgal(format!(
                        "Galgame choice {} 引用不存在的行，无法导出 WebGAL: line_id={}",
                        choice.id, choice.line_id
                    )));
                }
            }
            let target_scene_id = choice.target_scene_id.ok_or_else(|| {
                invalid_webgal(format!(
                    "Galgame choice {} 缺少 target_scene_id，无法导出 WebGAL",
                    choice.id
                ))
            })?;
            if !scene_file_by_id.contains_key(&target_scene_id) {
                return Err(invalid_webgal(format!(
                    "Galgame choice {} 指向不存在的场 {target_scene_id}，无法导出 WebGAL",
                    choice.id
                )));
            }
            let target_scene = detail
                .scenes
                .iter()
                .find(|candidate| candidate.scene.id == target_scene_id)
                .ok_or_else(|| {
                    invalid_webgal(format!(
                        "Galgame choice {} 指向详情中不存在的场 {target_scene_id}",
                        choice.id
                    ))
                })?;
            if target_scene.scene.entry_kind == GalgameSceneEntryKind::Branch {
                let return_scene_id = target_scene.scene.next_scene_id.ok_or_else(|| {
                    invalid_webgal(format!(
                        "Galgame choice {} 指向没有回线的 branch 场，无法导出 WebGAL",
                        choice.id
                    ))
                })?;
                let return_scene = detail
                    .scenes
                    .iter()
                    .find(|candidate| candidate.scene.id == return_scene_id)
                    .ok_or_else(|| {
                        invalid_webgal(format!(
                            "Galgame choice {} 的 branch 回线指向不存在的场 {return_scene_id}",
                            choice.id
                        ))
                    })?;
                if return_scene.scene.entry_kind != GalgameSceneEntryKind::Main
                    || return_scene.scene.scene_index <= scene.scene.scene_index
                {
                    return Err(invalid_webgal(format!(
                        "Galgame choice {} 的 branch 回线必须进入选择所在场之后的主线场，当前选择场={}, 回线场={}",
                        choice.id, scene.scene.scene_index, return_scene.scene.scene_index
                    )));
                }
            }
            choices_by_line
                .entry(choice.line_id)
                .or_default()
                .push(choice);
        }

        for line in scene
            .lines
            .iter()
            .filter(|line| line.line_type == GalgameLineType::Choice)
        {
            let choices = choices_by_line.get(&line.id).ok_or_else(|| {
                invalid_webgal(format!(
                    "Galgame choice 行 {} 缺少选项，无法导出 WebGAL",
                    line.id
                ))
            })?;
            if !(2..=3).contains(&choices.len()) {
                return Err(invalid_webgal(format!(
                    "Galgame choice 行 {} 选项数量必须是 2 到 3 个，无法导出 WebGAL: choice_count={}",
                    line.id,
                    choices.len()
                )));
            }
            let mut option_indexes = BTreeSet::new();
            for choice in choices {
                if choice.option_text.trim().is_empty() || choice.option_index <= 0 {
                    return Err(invalid_webgal(format!(
                        "Galgame choice {} 选项文本或序号无效，无法导出 WebGAL",
                        choice.id
                    )));
                }
                if !option_indexes.insert(choice.option_index) {
                    return Err(invalid_webgal(format!(
                        "Galgame choice {} option_index 重复，无法导出 WebGAL: line_id={}, option_index={}",
                        choice.id, choice.line_id, choice.option_index
                    )));
                }
            }
            let choice_count = i32::try_from(choices.len()).map_err(|_| {
                invalid_webgal(format!(
                    "Galgame choice 行 {} 的选项数量超过 i32 范围",
                    line.id
                ))
            })?;
            let expected_indexes = (1..=choice_count).collect::<BTreeSet<_>>();
            if option_indexes != expected_indexes {
                return Err(invalid_webgal(format!(
                    "Galgame choice 行 {} option_index 必须从 1 连续编号，无法导出 WebGAL",
                    line.id
                )));
            }
        }
    }
    Ok(())
}

fn webgal_config_text(detail: &GalgameScriptDetail) -> Result<String, AppError> {
    let title = &detail.title;
    if title.trim() != title
        || title
            .chars()
            .any(|ch| matches!(ch, ';' | '\r' | '\n' | '\t'))
        || title.contains(" -")
        || title.split('|').any(|part| part.trim() != part)
    {
        return Err(invalid_webgal(
            "WebGAL 配置无法无损表示游戏标题中的分号、控制空白、参数分隔符或带边界空格的竖线",
        ));
    }
    Ok(format!(
        "Game_name:{title};\nGame_key:a4n_galgame_{};\n",
        detail.id
    ))
}

fn read_webgal_data_file(data_dir: &Path, relative_path: &str) -> Result<Vec<u8>, AppError> {
    let raw = Path::new(relative_path);
    if raw.is_absolute() {
        return Err(invalid_webgal(
            "WebGAL 媒体文件必须是应用数据目录内的相对路径",
        ));
    }
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| invalid_webgal(format!("解析应用数据目录失败：{error}")))?;
    let canonical_file = data_dir
        .join(raw)
        .canonicalize()
        .map_err(|error| invalid_webgal(format!("解析 WebGAL 媒体路径失败：{error}")))?;
    if !canonical_file.starts_with(&canonical_data_dir) || !canonical_file.is_file() {
        return Err(invalid_webgal(
            "WebGAL 媒体文件不在应用数据目录内或不是文件",
        ));
    }
    std::fs::read(&canonical_file).map_err(|error| {
        invalid_webgal(format!(
            "读取 WebGAL 媒体文件失败: {}: {error}",
            canonical_file.display()
        ))
    })
}

fn invalid_webgal(message: impl Into<String>) -> AppError {
    AppError::InvalidState {
        current: "webgal_export".to_string(),
        operation: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::galgame::{GalgameLine, GalgameScene};
    use a4n_domain::galgame::{
        GalgameFacingV2, GalgameLinePresentationDocumentV3, GalgameStageLayerV2,
        GalgameStagePositionV2, GalgameVisibleCastMemberV2,
    };
    use serde_json::Value;

    fn scene(lines: Vec<GalgameLine>) -> GalgameSceneDetail {
        GalgameSceneDetail {
            scene: GalgameScene {
                adaptation_plan_id: None,
                adaptation_unit_id: None,
                id: 10,
                source_snapshot_chapter_id: 20,
                scene_index: 1,
                title: "测试场".to_string(),
                location: None,
                time_of_day: None,
                background_note: None,
                entry_kind: GalgameSceneEntryKind::Main,
                next_scene_id: None,
            },
            lines,
            choices: Vec::new(),
        }
    }

    fn line(id: i64, line_type: GalgameLineType, presentation_json: Value) -> GalgameLine {
        GalgameLine {
            id,
            scene_id: 10,
            line_index: 1,
            line_type,
            speaker_profile_id: None,
            text: "旁白".to_string(),
            expression: None,
            pose: None,
            position: None,
            presentation_json: serde_json::from_value(presentation_json).unwrap(),
            text_anchor_id: None,
        }
    }

    fn presentation(
        visible_cast: Vec<GalgameVisibleCastMemberV2>,
        enter: Vec<i64>,
        exit: Vec<i64>,
    ) -> Value {
        serde_json::to_value(GalgameLinePresentationDocumentV3 {
            schema_version: a4n_domain::galgame::GalgameLineDocumentSchemaVersion::V3,
            origin: a4n_domain::media_authoring::AdaptationOrigin::Historical,
            beat_ref: None,
            visible_cast,
            enter,
            exit,
            focus_character_profile_id: None,
            hold: false,
            player_knowledge: Vec::new(),
            cg: None,
            cg_note: None,
            cg_importance: None,
            transition: None,
            branch: None,
            branch_id: None,
            returns_to_scene_index: None,
            after_beat_ref: None,
        })
        .unwrap()
    }

    #[test]
    fn renders_all_visible_cast_members_on_narration_lines() {
        let visible_cast = vec![
            GalgameVisibleCastMemberV2 {
                character_profile_id: 47,
                expression: "serious".to_string(),
                pose: "watch".to_string(),
                position: GalgameStagePositionV2::Left,
                facing: GalgameFacingV2::Left,
                layer: GalgameStageLayerV2::Back,
            },
            GalgameVisibleCastMemberV2 {
                character_profile_id: 48,
                expression: "neutral".to_string(),
                pose: "toolbox".to_string(),
                position: GalgameStagePositionV2::Right,
                facing: GalgameFacingV2::Right,
                layer: GalgameStageLayerV2::Front,
            },
        ];
        let mut assets = WebgalExportAssets::default();
        for (profile_id, expression, pose, file) in [
            (47, "serious", "watch", "lin.webp"),
            (48, "neutral", "toolbox", "zhou.webp"),
        ] {
            assets.figures.insert(
                WebgalFigureKey {
                    profile_id,
                    expression: Some(expression.to_string()),
                    pose: Some(pose.to_string()),
                },
                WebgalExportAsset {
                    script_name: file.to_string(),
                    zip_path: format!("game/figure/{file}"),
                    data_path: format!("portraits/{file}"),
                },
            );
        }

        let script = render_webgal_scene(
            &scene(vec![
                line(
                    100,
                    GalgameLineType::Narration,
                    presentation(visible_cast, vec![47, 48], Vec::new()),
                ),
                line(
                    101,
                    GalgameLineType::Narration,
                    presentation(
                        vec![GalgameVisibleCastMemberV2 {
                            character_profile_id: 48,
                            expression: "neutral".to_string(),
                            pose: "toolbox".to_string(),
                            position: GalgameStagePositionV2::Right,
                            facing: GalgameFacingV2::Right,
                            layer: GalgameStageLayerV2::Front,
                        }],
                        Vec::new(),
                        vec![47],
                    ),
                ),
            ]),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &assets,
            &BTreeSet::from([47, 48]),
        )
        .unwrap();

        assert!(script.contains("changeFigure:lin.webp -id=a4n_profile_47 -left"));
        assert!(script.contains("changeFigure:zhou.webp -id=a4n_profile_48 -right"));
        assert!(script.contains("-zIndex=1"));
        assert!(script.contains("-zIndex=3"));
        assert!(script.contains("-transform={\"scale\":{\"x\":-1,\"y\":1}}"));
        assert!(script.contains(
            "setTransition: -target=a4n_profile_47 -exit=exit;\nchangeFigure:none -id=a4n_profile_47 -next;"
        ));
    }

    #[test]
    fn injects_choice_text_after_webgal_command_parsing() {
        let option_text = "继续调查 - 暂缓验证:甲|乙";
        let choices = [
            GalgameChoice {
                id: 201,
                line_id: 100,
                option_index: 1,
                option_text: option_text.to_string(),
                target_scene_id: Some(20),
            },
            GalgameChoice {
                id: 202,
                line_id: 100,
                option_index: 2,
                option_text: "暂时离开".to_string(),
                target_scene_id: Some(30),
            },
        ];
        let choices_by_line = BTreeMap::from([(100, choices.iter().collect())]);
        let script = render_webgal_scene(
            &scene(vec![line(
                100,
                GalgameLineType::Choice,
                presentation(Vec::new(), Vec::new(), Vec::new()),
            )]),
            &BTreeMap::from([
                (20, "scene_002_20.txt".to_string()),
                (30, "scene_003_30.txt".to_string()),
            ]),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &choices_by_line,
            &WebgalExportAssets::default(),
            &BTreeSet::new(),
        )
        .unwrap();
        let escaped = webgal_choice_text(option_text).unwrap();
        let literal = webgal_string_literal(&escaped);

        assert!(script.contains(&format!("setVar:__a4n_choice_100_1=({literal});")));
        assert!(script.contains(
            "choose:{__a4n_choice_100_1}:scene_002_20.txt|{__a4n_choice_100_2}:scene_003_30.txt;"
        ));
    }
}
