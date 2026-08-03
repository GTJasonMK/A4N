use std::collections::HashSet;

use a4n_core::{
    error::AppError,
    manga::{MangaPagePlanV3, MangaPanelSizeV2, MangaShootingBeatRefV3},
    shooting_script::ShootingCoverageV1,
};
use a4n_db::shooting_script::ParsedChapterShootingScript;
use serde_json::Value;

pub(crate) fn parse_manga_page_plan(
    json: &Value,
    shooting_script: &ParsedChapterShootingScript,
    context: &str,
) -> Result<MangaPagePlanV3, AppError> {
    let mut plan: MangaPagePlanV3 = serde_json::from_value(json.clone()).map_err(|error| {
        AppError::InvalidArgument(format!("{context} 不符合 manga_page_plan_v3 合同: {error}"))
    })?;
    sync_page_beat_refs_from_panels(&mut plan);
    plan.validate()
        .map_err(|error| AppError::InvalidArgument(format!("{context}: {error}")))?;
    if plan.shooting_script_id != shooting_script.record.id
        || plan.shooting_script_revision != shooting_script.record.revision
    {
        return Err(AppError::InvalidArgument(format!(
            "{context} 拍摄剧本身份不一致: expected id={} revision={}, actual id={} revision={}",
            shooting_script.record.id,
            shooting_script.record.revision,
            plan.shooting_script_id,
            plan.shooting_script_revision
        )));
    }
    validate_shooting_beat_coverage(&plan, shooting_script, context)?;
    Ok(plan)
}

fn sync_page_beat_refs_from_panels(plan: &mut MangaPagePlanV3) {
    for page in &mut plan.pages {
        for panel in &mut page.panels {
            let mut seen = HashSet::new();
            panel.beat_refs.retain(|beat_ref| seen.insert(*beat_ref));
            if panel.panel_number == page.dominant_panel_number {
                panel.size = MangaPanelSizeV2::Dominant;
            } else if panel.size == MangaPanelSizeV2::Dominant {
                panel.size = MangaPanelSizeV2::Large;
            }
        }
        let mut seen = HashSet::new();
        page.beat_refs = page
            .panels
            .iter()
            .flat_map(|panel| panel.beat_refs.iter().copied())
            .filter(|beat_ref| seen.insert(*beat_ref))
            .collect();
    }
}

pub(crate) fn page_plan_pages<'a>(json: &'a Value, context: &str) -> Result<&'a [Value], AppError> {
    let pages = json
        .get("pages")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.pages 必须是已验证的数组")))?;
    if pages.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context}.pages 不能为空"
        )));
    }
    Ok(pages)
}

pub(crate) fn page_plan_page_number(page: &Value, context: &str) -> Result<i32, AppError> {
    let value = page
        .get("page_number")
        .and_then(Value::as_u64)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.page_number 必须是正整数")))?;
    i32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.page_number 超出 i32 范围")))
}

fn validate_shooting_beat_coverage(
    plan: &MangaPagePlanV3,
    shooting_script: &ParsedChapterShootingScript,
    context: &str,
) -> Result<(), AppError> {
    let available = shooting_script
        .script
        .scenes
        .iter()
        .flat_map(|scene| {
            scene.shots.iter().flat_map(|shot| {
                shot.performance_beats.iter().map(|beat| {
                    (
                        MangaShootingBeatRefV3 {
                            scene_index: scene.scene_index,
                            shot_index: shot.shot_index,
                            beat_index: beat.beat_index,
                        },
                        (shot.coverage, beat.required),
                    )
                })
            })
        })
        .collect::<Vec<_>>();
    let available_refs = available
        .iter()
        .map(|(beat_ref, _)| *beat_ref)
        .collect::<HashSet<_>>();
    let used_refs = plan
        .pages
        .iter()
        .flat_map(|page| page.beat_refs.iter().copied())
        .collect::<HashSet<_>>();
    for beat_ref in &used_refs {
        if !available_refs.contains(beat_ref) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 引用了不存在的拍摄节拍: scene_index={}, shot_index={}, beat_index={}",
                beat_ref.scene_index, beat_ref.shot_index, beat_ref.beat_index
            )));
        }
    }
    for (beat_ref, (coverage, required)) in available {
        if (required || coverage == ShootingCoverageV1::MustShow) && !used_refs.contains(&beat_ref)
        {
            return Err(AppError::InvalidArgument(format!(
                "{context} 遗漏 must_show 拍摄节拍: scene_index={}, shot_index={}, beat_index={}",
                beat_ref.scene_index, beat_ref.shot_index, beat_ref.beat_index
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use a4n_core::manga::{MangaPagePlanV3, MangaPanelSizeV2};
    use serde_json::json;

    use super::sync_page_beat_refs_from_panels;

    #[test]
    fn page_beat_refs_are_derived_from_panels() {
        let mut plan: MangaPagePlanV3 = serde_json::from_value(json!({
            "schema_version": "manga_page_plan_v3",
            "shooting_script_id": 1,
            "shooting_script_revision": 1,
            "pages": [{
                "page_number": 1,
                "beat_refs": [{"scene_index": 1, "shot_index": 1, "beat_index": 1}],
                "role": "opening",
                "pacing": "slow",
                "importance": "high",
                "reading_flow": "z_path",
                "page_goal": "建立异常",
                "visual_thesis": "用门框压缩空间",
                "entry_state": "门关闭",
                "exit_state": "门打开",
                "dominant_panel_number": 1,
                "panels": [
                    {
                        "panel_number": 1,
                        "beat_refs": [{"scene_index": 1, "shot_index": 1, "beat_index": 1}],
                        "function": "establish",
                        "size": "large",
                        "shot_scale": "wide",
                        "camera_angle": "eye_level",
                        "participants": ["角色"],
                        "composition": "门框围住角色",
                        "observable_action": "角色推门",
                        "reaction": null,
                        "spatial_relationship": "角色在门外",
                        "continuity_from_previous": null,
                        "text_layers": []
                    },
                    {
                        "panel_number": 2,
                        "beat_refs": [
                            {"scene_index": 1, "shot_index": 2, "beat_index": 1},
                            {"scene_index": 1, "shot_index": 2, "beat_index": 1}
                        ],
                        "function": "reaction",
                        "size": "dominant",
                        "shot_scale": "close_up",
                        "camera_angle": "eye_level",
                        "participants": ["角色"],
                        "composition": "角色眼睛位于画面中央",
                        "observable_action": "角色停住",
                        "reaction": "角色屏住呼吸",
                        "spatial_relationship": "角色仍在门外",
                        "continuity_from_previous": "承接推门动作",
                        "text_layers": []
                    }
                ]
            }]
        }))
        .expect("page plan contract");

        sync_page_beat_refs_from_panels(&mut plan);

        assert_eq!(
            serde_json::to_value(&plan.pages[0].beat_refs).expect("beat refs"),
            json!([
                {"scene_index": 1, "shot_index": 1, "beat_index": 1},
                {"scene_index": 1, "shot_index": 2, "beat_index": 1}
            ])
        );
        assert_eq!(plan.pages[0].panels[1].beat_refs.len(), 1);
        assert_eq!(plan.pages[0].panels[0].size, MangaPanelSizeV2::Dominant);
        assert_eq!(plan.pages[0].panels[1].size, MangaPanelSizeV2::Large);
        plan.validate().expect("normalized plan should validate");
    }
}
