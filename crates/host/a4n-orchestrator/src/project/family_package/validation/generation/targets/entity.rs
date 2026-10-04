use std::collections::{HashMap, HashSet};

use crate::project::snapshot_profile_key;

use super::galgame::validate_generation_spec_galgame_scene_context;

pub(in super::super) fn validate_generation_spec_target_entity(
    spec: &a4n_domain::project::ProjectFamilyGenerationSpecRecord,
    source_snapshot_id: i64,
    snapshot_chapters: &HashMap<
        i64,
        &a4n_domain::project::ProjectFamilySourceSnapshotChapterRecord,
    >,
    snapshot_profiles: &HashMap<
        (i64, String),
        &a4n_domain::project::ProjectFamilySourceSnapshotProfileRecord,
    >,
    chapter_video_prompt_ids: &HashSet<i64>,
    video_segment_ids: &HashSet<i64>,
    assembly_timeline_ids: &HashSet<i64>,
    galgame_scripts: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameScriptRecord>,
    galgame_scenes: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameSceneRecord>,
    galgame_lines: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameLineRecord>,
) -> Result<(), String> {
    let Some(target_entity_id) = spec.target_entity_id else {
        return Ok(());
    };
    match spec.target_entity_kind.as_str() {
        "character" | "character_profile" | "object" | "scene"
            if !snapshot_profiles.contains_key(&(
                source_snapshot_id,
                snapshot_profile_key(&spec.target_entity_kind, target_entity_id),
            )) =>
        {
            return Err(format!(
                "项目族导入包生成意图目标实体不属于源快照: generation_spec_id={}, target_entity_kind={}, target_entity_id={}, source_snapshot_id={}",
                spec.id, spec.target_entity_kind, target_entity_id, source_snapshot_id
            ));
        }
        "chapter" => {
            let snapshot_chapter = snapshot_chapters.get(&target_entity_id).ok_or_else(|| {
                format!(
                    "项目族导入包生成意图章节目标不存在: generation_spec_id={}, target_entity_id={target_entity_id}",
                    spec.id
                )
            })?;
            if snapshot_chapter.snapshot_id != source_snapshot_id {
                return Err(format!(
                    "项目族导入包生成意图章节目标不属于源快照: generation_spec_id={}, target_entity_id={}, source_snapshot_id={}",
                    spec.id, target_entity_id, source_snapshot_id
                ));
            }
        }
        "chapter_video_prompt" if !chapter_video_prompt_ids.contains(&target_entity_id) => {
            return Err(format!(
                "项目族导入包生成意图章节视频提示词目标不存在: generation_spec_id={}, target_entity_id={target_entity_id}",
                spec.id
            ));
        }
        "segment" if !video_segment_ids.contains(&target_entity_id) => {
            return Err(format!(
                "项目族导入包生成意图视频片段目标不存在: generation_spec_id={}, target_entity_id={target_entity_id}",
                spec.id
            ));
        }
        "assembly" if !assembly_timeline_ids.contains(&target_entity_id) => {
            return Err(format!(
                "项目族导入包生成意图成片时间线目标不存在: generation_spec_id={}, target_entity_id={target_entity_id}",
                spec.id
            ));
        }
        "galgame_scene" => {
            let scene = galgame_scenes.get(&target_entity_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包生成意图 Galgame 场目标不存在: generation_spec_id={}, target_entity_id={target_entity_id}",
                    spec.id
                )
            })?;
            validate_generation_spec_galgame_scene_context(
                spec,
                source_snapshot_id,
                scene,
                galgame_scripts,
                snapshot_chapters,
            )?;
        }
        "galgame_line" => {
            let line = galgame_lines.get(&target_entity_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包生成意图 Galgame 行目标不存在: generation_spec_id={}, target_entity_id={target_entity_id}",
                    spec.id
                )
            })?;
            let scene = galgame_scenes.get(&line.scene_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包生成意图 Galgame 行所属场不存在: generation_spec_id={}, target_entity_id={}, scene_id={}",
                    spec.id, target_entity_id, line.scene_id
                )
            })?;
            validate_generation_spec_galgame_scene_context(
                spec,
                source_snapshot_id,
                scene,
                galgame_scripts,
                snapshot_chapters,
            )?;
        }
        "segment" | "assembly" => {}
        _ => {}
    }
    Ok(())
}
