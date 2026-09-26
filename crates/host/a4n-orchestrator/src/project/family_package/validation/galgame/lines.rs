use super::super::*;
use super::rules::validate_galgame_line_semantics;

pub(super) fn validate_galgame_lines(
    package: &ProjectFamilyPackage,
    scripts: &HashMap<i64, &galgame_script::Model>,
    scenes: &HashMap<i64, &galgame_scene::Model>,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
    text_anchors: &HashMap<i64, &text_anchor::Model>,
    character_profile_ids: &HashSet<i64>,
) -> Result<(), String> {
    let mut lines_by_scene = HashMap::<i64, Vec<&galgame_line::Model>>::new();
    for line in &package.galgame_lines {
        let scene = scenes.get(&line.scene_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 行引用了不存在的场: line_id={}, scene_id={}",
                line.id, line.scene_id
            )
        })?;
        validate_galgame_line_semantics(line)?;
        if let Some(profile_id) = line.speaker_profile_id
            && !character_profile_ids.contains(&profile_id)
        {
            return Err(format!(
                "项目族导入包 Galgame 行 speaker_profile_id 不存在: line_id={}, speaker_profile_id={profile_id}",
                line.id
            ));
        }
        if let Some(anchor_id) = line.text_anchor_id {
            let anchor = text_anchors.get(&anchor_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 行 text_anchor_id 不存在: line_id={}, text_anchor_id={anchor_id}",
                    line.id
                )
            })?;
            let script = scripts.get(&scene.script_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 行所属场引用了不存在的剧本: line_id={}, scene_id={}, script_id={}",
                    line.id, scene.id, scene.script_id
                )
            })?;
            let chapter = snapshot_chapters
                .get(&scene.source_snapshot_chapter_id)
                .ok_or_else(|| {
                    format!(
                        "项目族导入包 Galgame 场源快照章节不存在: scene_id={}",
                        scene.id
                    )
                })?;
            if anchor.project_id != script.root_project_id
                || anchor.source_snapshot_id != Some(chapter.snapshot_id)
                || anchor.source_snapshot_chapter_id != Some(scene.source_snapshot_chapter_id)
            {
                return Err(format!(
                    "项目族导入包 Galgame 行 text_anchor_id 不属于行所属场源快照章节: line_id={}, text_anchor_id={anchor_id}, source_snapshot_chapter_id={:?}, expected_source_snapshot_chapter_id={}",
                    line.id, anchor.source_snapshot_chapter_id, scene.source_snapshot_chapter_id
                ));
            }
        }
        lines_by_scene.entry(line.scene_id).or_default().push(line);
    }

    for (scene_id, scene_lines) in &lines_by_scene {
        let mut line_indexes = BTreeSet::new();
        for line in scene_lines {
            if !line_indexes.insert(line.line_index) {
                return Err(format!(
                    "项目族导入包 Galgame 行 line_index 重复: scene_id={}, line_id={}, line_index={}",
                    scene_id, line.id, line.line_index
                ));
            }
        }
        let line_count = i32::try_from(scene_lines.len()).map_err(|_| {
            format!(
                "项目族导入包 Galgame 场景 {scene_id} 的行数量超过 i32 范围：{}",
                scene_lines.len()
            )
        })?;
        let expected_indexes = (1..=line_count).collect::<BTreeSet<_>>();
        if line_indexes != expected_indexes {
            return Err(format!(
                "项目族导入包 Galgame 行 line_index 必须从 1 连续编号: scene_id={}, line_indexes={:?}, expected={:?}",
                scene_id, line_indexes, expected_indexes
            ));
        }
    }
    Ok(())
}
