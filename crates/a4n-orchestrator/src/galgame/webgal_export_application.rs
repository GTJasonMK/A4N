use std::collections::{BTreeMap, BTreeSet};

use a4n_api_contract::export::ExportFileData;
use a4n_core::{error::AppError, models::character::character_portrait};
use a4n_export_core::{ExportZipEntryBytes, ExportZipPackageInput, render_zip_package};
use base64::Engine as _;

use crate::OrchestratorContext;

use super::{
    GalgameChoice, GalgameLine, GalgameLineType, GalgameMediaRole, GalgameMediaTargetKind,
    GalgameSceneDetail, GalgameSceneEntryKind, GalgameScriptDetail,
    webgal_export_rules::{
        media_file_extension, media_file_extension_for_webgal_role, sanitize_export_file_stem,
        webgal_arg, webgal_narration, webgal_speaker, webgal_text, webgal_transition_args,
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
    portraits: &[character_portrait::Model],
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
    validate_webgal_choice_contract(detail, &scene_file_by_id)?;
    let assets = webgal_export_assets(detail, portraits)?;
    let first_scene = scene_file_by_id
        .get(&detail.scenes[0].scene.id)
        .ok_or_else(|| invalid_webgal("Galgame 首场导出文件名缺失"))?;

    let mut entries = vec![ExportZipEntryBytes {
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
        )?;
        entries.push(ExportZipEntryBytes {
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
            entries.push(ExportZipEntryBytes {
                path: asset.zip_path.clone(),
                bytes: read_webgal_data_file(data_dir, &asset.data_path)?,
                label: format!("WebGAL 媒体 {}", asset.zip_path),
            });
        }
    }

    render_zip_package(ExportZipPackageInput {
        manifest_path: "game/config.txt".to_string(),
        manifest_bytes: webgal_config_text(detail).into_bytes(),
        manifest_label: "WebGAL 配置".to_string(),
        entries,
        package_label: "WebGAL zip 包".to_string(),
    })
    .map_err(|error| invalid_webgal(error.to_string()))
}

fn render_webgal_scene(
    scene: &GalgameSceneDetail,
    scene_file_by_id: &BTreeMap<i64, String>,
    scene_entry_by_id: &BTreeMap<i64, GalgameSceneEntryKind>,
    speaker_name_by_id: &BTreeMap<i64, String>,
    choices_by_line: &BTreeMap<i64, Vec<&GalgameChoice>>,
    assets: &WebgalExportAssets,
) -> Result<String, AppError> {
    let mut output = String::new();
    let mut current_figure_by_position = BTreeMap::<String, String>::new();
    if let Some(background) = assets.scene_backgrounds.get(&scene.scene.id) {
        output.push_str(&format!("changeBg:{} -next;\n", background.script_name));
    }
    for line in &scene.lines {
        let transition_args = webgal_transition_args(line.presentation_json.transition);
        if let Some(bgm) = assets.line_bgms.get(&line.id) {
            output.push_str(&format!("bgm:{};\n", bgm.script_name));
        }
        if let Some(sfx) = assets.line_sfx.get(&line.id) {
            output.push_str(&format!("playEffect:{} -next;\n", sfx.script_name));
        }
        if let Some(cg) = assets.line_cgs.get(&line.id) {
            output.push_str(&format!(
                "changeBg:{}{} -next;\n",
                cg.script_name, transition_args
            ));
            output.push_str(&format!(
                "unlockCg:{} -name={};\n",
                cg.script_name,
                webgal_arg(&format!("{} #{} CG", scene.scene.title, line.line_index))
            ));
        }
        if let Some((position, figure)) = webgal_line_figure(line, assets) {
            let changed = current_figure_by_position
                .get(position)
                .is_none_or(|current| current != &figure.script_name);
            if changed {
                output.push_str(&format!(
                    "changeFigure:{} -{} -next;\n",
                    figure.script_name, position
                ));
                current_figure_by_position.insert(position.to_string(), figure.script_name.clone());
            }
        }
        match line.line_type {
            GalgameLineType::Dialogue | GalgameLineType::Monologue => {
                let speaker_profile_id = line.speaker_profile_id.ok_or_else(|| {
                    invalid_webgal(format!(
                        "Galgame {} 行 {} 缺少 speaker_profile_id，无法导出 WebGAL",
                        line.line_type, line.id
                    ))
                })?;
                let speaker = speaker_name_by_id
                    .get(&speaker_profile_id)
                    .map(|name| webgal_speaker(name))
                    .ok_or_else(|| {
                        invalid_webgal(format!(
                            "Galgame {} 行 {} 绑定的角色档案 {} 不存在，无法导出 WebGAL",
                            line.line_type, line.id, speaker_profile_id
                        ))
                    })?;
                output.push_str(&format!("{}:{};\n", speaker, webgal_text(&line.text)));
            }
            GalgameLineType::Narration | GalgameLineType::StageDirection => {
                output.push_str(&format!("{};\n", webgal_narration(&line.text)));
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
                        Ok(format!("{}:{}", webgal_arg(&choice.option_text), target))
                    })
                    .collect::<Result<Vec<_>, AppError>>()?
                    .join("|");
                output.push_str(&format!("choose:{options};\n"));
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

fn webgal_line_figure<'a>(
    line: &GalgameLine,
    assets: &'a WebgalExportAssets,
) -> Option<(&'static str, &'a WebgalExportAsset)> {
    let profile_id = line.speaker_profile_id?;
    let position = line.position.map(|position| position.as_str())?;
    let key = WebgalFigureKey {
        profile_id,
        expression: line.expression.clone(),
        pose: line.pose.clone(),
    };
    if key.expression.is_some() || key.pose.is_some() {
        return assets.figures.get(&key).map(|asset| (position, asset));
    }
    assets
        .figures
        .get(&WebgalFigureKey {
            profile_id,
            expression: None,
            pose: None,
        })
        .map(|asset| (position, asset))
}

fn webgal_export_assets(
    detail: &GalgameScriptDetail,
    portraits: &[character_portrait::Model],
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

fn webgal_config_text(detail: &GalgameScriptDetail) -> String {
    format!(
        "Game_name:{};\nGame_key:a4n_galgame_{};\n",
        webgal_arg(&detail.title),
        detail.id
    )
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
