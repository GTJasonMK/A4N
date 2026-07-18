use super::assets::webgal_line_figure;
use super::contracts::WebgalExportAssets;
use super::rules::{
    webgal_arg, webgal_narration, webgal_speaker, webgal_text, webgal_transition_args,
};
use a4n_core::models::galgame::galgame_choice;
use a4n_orchestrator::galgame::{
    GalgameSceneDetail, LINE_TYPE_CHOICE, LINE_TYPE_DIALOGUE, LINE_TYPE_MONOLOGUE,
    LINE_TYPE_NARRATION, LINE_TYPE_STAGE_DIRECTION, SCENE_ENTRY_BRANCH, SCENE_ENTRY_MAIN,
};
use std::collections::BTreeMap;

pub(super) fn render_webgal_scene(
    scene: &GalgameSceneDetail,
    scene_file_by_id: &BTreeMap<i64, String>,
    scene_entry_by_id: &BTreeMap<i64, String>,
    speaker_name_by_id: &BTreeMap<i64, String>,
    choices_by_line: &BTreeMap<i64, Vec<&galgame_choice::Model>>,
    assets: &WebgalExportAssets,
) -> Result<String, String> {
    let mut out = String::new();
    let mut current_figure_by_position = BTreeMap::<String, String>::new();
    if let Some(background) = assets.scene_backgrounds.get(&scene.scene.id) {
        out.push_str(&format!("changeBg:{} -next;\n", background.script_name));
    }
    for line in &scene.lines {
        let transition_args = webgal_transition_args(&line.presentation_json, line.id)?;
        if let Some(bgm) = assets.line_bgms.get(&line.id) {
            out.push_str(&format!("bgm:{};\n", bgm.script_name));
        }
        if let Some(sfx) = assets.line_sfx.get(&line.id) {
            out.push_str(&format!("playEffect:{} -next;\n", sfx.script_name));
        }
        if let Some(cg) = assets.line_cgs.get(&line.id) {
            out.push_str(&format!(
                "changeBg:{}{} -next;\n",
                cg.script_name, transition_args
            ));
            out.push_str(&format!(
                "unlockCg:{} -name={};\n",
                cg.script_name,
                webgal_arg(&format!("{} #{} CG", scene.scene.title, line.line_index))
            ));
        }
        if let Some((position, figure)) = webgal_line_figure(line, assets)? {
            let changed = current_figure_by_position
                .get(position)
                .map(|current| current != &figure.script_name)
                .unwrap_or(true);
            if changed {
                out.push_str(&format!(
                    "changeFigure:{} -{} -next;\n",
                    figure.script_name, position
                ));
                current_figure_by_position.insert(position.to_string(), figure.script_name.clone());
            }
        }
        match line.line_type.as_str() {
            LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE => {
                let speaker_profile_id = line.speaker_profile_id.ok_or_else(|| {
                    format!(
                        "Galgame {} 行 {} 缺少 speaker_profile_id，无法导出 WebGAL",
                        line.line_type, line.id
                    )
                })?;
                let speaker = speaker_name_by_id
                    .get(&speaker_profile_id)
                    .map(|name| webgal_speaker(name))
                    .ok_or_else(|| {
                        format!(
                            "Galgame {} 行 {} 绑定的角色档案 {} 不存在，无法导出 WebGAL",
                            line.line_type, line.id, speaker_profile_id
                        )
                    })?;
                out.push_str(&format!("{}:{};\n", speaker, webgal_text(&line.text)));
            }
            LINE_TYPE_NARRATION | LINE_TYPE_STAGE_DIRECTION => {
                out.push_str(&format!("{};\n", webgal_narration(&line.text)));
            }
            LINE_TYPE_CHOICE => {
                let choices = choices_by_line
                    .get(&line.id)
                    .filter(|items| !items.is_empty())
                    .ok_or_else(|| {
                        format!(
                            "Galgame choice 行 {} 缺少 galgame_choice 选项，无法导出 WebGAL",
                            line.id
                        )
                    })?;
                let mut sorted_choices = choices.clone();
                sorted_choices.sort_by_key(|choice| choice.option_index);
                let options = sorted_choices
                    .iter()
                    .map(|choice| {
                        let target_scene_id = choice.target_scene_id.ok_or_else(|| {
                            format!(
                                "Galgame choice {} 缺少 target_scene_id，无法导出 WebGAL",
                                choice.id
                            )
                        })?;
                        let target = scene_file_by_id.get(&target_scene_id).ok_or_else(|| {
                            format!(
                                "Galgame choice {} 指向不存在的场 {}，无法导出 WebGAL",
                                choice.id, target_scene_id
                            )
                        })?;
                        Ok(format!("{}:{}", webgal_arg(&choice.option_text), target))
                    })
                    .collect::<Result<Vec<_>, String>>()?
                    .join("|");
                out.push_str(&format!("choose:{};\n", options));
            }
            other => {
                return Err(format!(
                    "Galgame 行 {} line_type 无效，无法导出 WebGAL: {}",
                    line.id, other
                ));
            }
        }
    }
    match scene.scene.next_scene_id {
        Some(next_scene_id) => {
            let next_file = scene_file_by_id.get(&next_scene_id).ok_or_else(|| {
                format!(
                    "Galgame 场 {} next_scene_id 指向不存在的场 {}，无法导出 WebGAL",
                    scene.scene.id, next_scene_id
                )
            })?;
            if scene.scene.entry_kind == SCENE_ENTRY_BRANCH
                && scene_entry_by_id
                    .get(&next_scene_id)
                    .map(|entry_kind| entry_kind.as_str())
                    != Some(SCENE_ENTRY_MAIN)
            {
                return Err(format!(
                    "Galgame branch 场 {} next_scene_id 必须指向主线场，当前指向 {}",
                    scene.scene.id, next_scene_id
                ));
            }
            out.push_str(&format!("changeScene:{};\n", next_file));
        }
        None if scene.scene.entry_kind == SCENE_ENTRY_BRANCH => {
            return Err(format!(
                "Galgame branch 场 {} 缺少 next_scene_id，无法导出 WebGAL",
                scene.scene.id
            ));
        }
        None => {}
    }
    Ok(out)
}
