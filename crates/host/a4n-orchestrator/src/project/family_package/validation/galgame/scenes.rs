use super::super::*;
use super::rules::{validate_galgame_participants_json, validate_galgame_scene_entry_kind};

pub(super) fn validate_galgame_scenes(
    package: &ProjectFamilyPackage,
    scripts: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameScriptRecord>,
    scenes: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameSceneRecord>,
    snapshot_chapters: &HashMap<
        i64,
        &a4n_domain::project::ProjectFamilySourceSnapshotChapterRecord,
    >,
    character_profile_ids: &HashSet<i64>,
) -> Result<(), String> {
    for scene in &package.galgame_scenes {
        let script = scripts.get(&scene.script_id).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 场引用了不存在的剧本: scene_id={}, script_id={}",
                scene.id, scene.script_id
            )
        })?;
        if scene.scene_index <= 0 {
            return Err(format!(
                "项目族导入包 Galgame 场 scene_index 必须是正整数: scene_id={}, scene_index={}",
                scene.id, scene.scene_index
            ));
        }
        if scene.title.trim().is_empty() {
            return Err(format!(
                "项目族导入包 Galgame 场 title 不能为空: scene_id={}",
                scene.id
            ));
        }
        validate_galgame_scene_entry_kind(scene)?;
        let snapshot_chapter = snapshot_chapters
            .get(&scene.source_snapshot_chapter_id)
            .ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 场引用了不存在的源快照章节: scene_id={}, source_snapshot_chapter_id={}",
                    scene.id, scene.source_snapshot_chapter_id
                )
            })?;
        if snapshot_chapter.root_project_id != script.root_project_id {
            return Err(format!(
                "项目族导入包 Galgame 场源快照章节与剧本根项目不一致: scene_id={}",
                scene.id
            ));
        }
        match (
            scene.adaptation_plan_id,
            scene.adaptation_unit_id.as_deref(),
        ) {
            (Some(id), Some(unit)) => {
                let document = package
                    .authoring_documents
                    .iter()
                    .find(|row| row.id == id)
                    .ok_or("游戏场景引用的本地改编计划不存在")?;
                if document.project_id != script.project_id
                    || document.kind != "adaptation_plan"
                    || document.source_snapshot_id != snapshot_chapter.snapshot_id
                    || document.source_chapter_id != Some(snapshot_chapter.source_chapter_id)
                    || !matches!(document.status.as_str(), "adopted" | "superseded")
                {
                    return Err("游戏场景与本地改编计划的归属或来源不一致".into());
                }
                let plan: a4n_domain::media_authoring::AdaptationPlanV1 =
                    serde_json::from_value(document.document_json.clone())
                        .map_err(|error| error.to_string())?;
                if !plan.units.iter().any(|item| item.id == unit) {
                    return Err("游戏场景引用的本地单元不存在".into());
                }
            }
            (None, None) if scene.entry_kind == "branch" => {}
            _ => return Err("游戏场景缺少完整本地改编来源".into()),
        }
        if let Some(next_scene_id) = scene.next_scene_id
            && !scenes.contains_key(&next_scene_id)
        {
            return Err(format!(
                "项目族导入包 Galgame 场 next_scene_id 不存在: scene_id={}, next_scene_id={next_scene_id}",
                scene.id
            ));
        }
        if scene.next_scene_id == Some(scene.id) {
            return Err(format!(
                "项目族导入包 Galgame 场 next_scene_id 不能指向自身: scene_id={}",
                scene.id
            ));
        }
        if let Some(next_scene_id) = scene.next_scene_id {
            let next_scene = scenes.get(&next_scene_id).ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 场 next_scene_id 不存在: scene_id={}, next_scene_id={next_scene_id}",
                    scene.id
                )
            })?;
            if next_scene.script_id != scene.script_id {
                return Err(format!(
                    "项目族导入包 Galgame 场 next_scene_id 不属于同一剧本: scene_id={}, next_scene_id={next_scene_id}",
                    scene.id
                ));
            }
        }
        if scene.entry_kind == "branch" {
            let Some(next_scene_id) = scene.next_scene_id else {
                return Err(format!(
                    "项目族导入包 Galgame branch 场缺少 next_scene_id: scene_id={}",
                    scene.id
                ));
            };
            let next_scene = scenes.get(&next_scene_id).ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 场 next_scene_id 不存在: scene_id={}, next_scene_id={next_scene_id}",
                    scene.id
                )
            })?;
            if next_scene.entry_kind != "main" {
                return Err(format!(
                    "项目族导入包 Galgame branch 场 next_scene_id 必须指向主线场: scene_id={}, next_scene_id={next_scene_id}",
                    scene.id
                ));
            }
        }
        validate_galgame_participants_json(
            scene.id,
            &scene.participants_json,
            character_profile_ids,
        )?;
    }
    Ok(())
}
