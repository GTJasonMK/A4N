use a4n_core::models::galgame::galgame_choice;
use a4n_orchestrator::galgame::{
    GalgameScriptDetail, LINE_TYPE_CHOICE, SCENE_ENTRY_BRANCH, SCENE_ENTRY_MAIN,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_webgal_scene_entry_kinds(
    detail: &GalgameScriptDetail,
) -> Result<(), String> {
    for scene in &detail.scenes {
        match scene.scene.entry_kind.as_str() {
            SCENE_ENTRY_MAIN | SCENE_ENTRY_BRANCH => {}
            other => {
                return Err(format!(
                    "Galgame 场 {} entry_kind 无效，无法导出 WebGAL: {}",
                    scene.scene.id, other
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_webgal_choice_contract(
    detail: &GalgameScriptDetail,
    scene_file_by_id: &BTreeMap<i64, String>,
) -> Result<(), String> {
    for scene in &detail.scenes {
        let line_type_by_id = scene
            .lines
            .iter()
            .map(|line| (line.id, line.line_type.as_str()))
            .collect::<BTreeMap<_, _>>();
        let mut choices_by_line = BTreeMap::<i64, Vec<&galgame_choice::Model>>::new();
        for choice in &scene.choices {
            match line_type_by_id.get(&choice.line_id).copied() {
                Some(LINE_TYPE_CHOICE) => {}
                Some(line_type) => {
                    return Err(format!(
                        "Galgame choice {} 挂在非 choice 行下，无法导出 WebGAL: line_id={}, line_type={}",
                        choice.id, choice.line_id, line_type
                    ));
                }
                None => {
                    return Err(format!(
                        "Galgame choice {} 引用不存在的行，无法导出 WebGAL: line_id={}",
                        choice.id, choice.line_id
                    ));
                }
            }
            let target_scene_id = choice.target_scene_id.ok_or_else(|| {
                format!(
                    "Galgame choice {} 缺少 target_scene_id，无法导出 WebGAL",
                    choice.id
                )
            })?;
            if !scene_file_by_id.contains_key(&target_scene_id) {
                return Err(format!(
                    "Galgame choice {} 指向不存在的场 {}，无法导出 WebGAL",
                    choice.id, target_scene_id
                ));
            }
            choices_by_line
                .entry(choice.line_id)
                .or_default()
                .push(choice);
        }

        for line in scene
            .lines
            .iter()
            .filter(|line| line.line_type == LINE_TYPE_CHOICE)
        {
            let choices = choices_by_line
                .get(&line.id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            if !(2..=3).contains(&choices.len()) {
                return Err(format!(
                    "Galgame choice 行 {} 选项数量必须是 2 到 3 个，无法导出 WebGAL: choice_count={}",
                    line.id,
                    choices.len()
                ));
            }
            let mut option_indexes = BTreeSet::new();
            for choice in choices {
                if choice.option_text.trim().is_empty() {
                    return Err(format!(
                        "Galgame choice {} option_text 不能为空，无法导出 WebGAL",
                        choice.id
                    ));
                }
                if choice.option_index <= 0 {
                    return Err(format!(
                        "Galgame choice {} option_index 必须为正数，无法导出 WebGAL: option_index={}",
                        choice.id, choice.option_index
                    ));
                }
                if !option_indexes.insert(choice.option_index) {
                    return Err(format!(
                        "Galgame choice {} option_index 重复，无法导出 WebGAL: line_id={}, option_index={}",
                        choice.id, choice.line_id, choice.option_index
                    ));
                }
            }
            let choice_count = i32::try_from(choices.len()).map_err(|_| {
                format!(
                    "Galgame choice 行 {} 的选项数量超过 i32 范围：{}",
                    line.id,
                    choices.len()
                )
            })?;
            let expected_indexes = (1..=choice_count).collect::<BTreeSet<_>>();
            if option_indexes != expected_indexes {
                return Err(format!(
                    "Galgame choice 行 {} option_index 必须从 1 连续编号，无法导出 WebGAL: option_indexes={:?}, expected={:?}",
                    line.id, option_indexes, expected_indexes
                ));
            }
        }
    }
    Ok(())
}
