use std::collections::HashSet;

use a4n_core::{
    error::AppError,
    manga::{MangaPagePlanV3, MangaShootingBeatRefV3},
    shooting_script::ShootingCoverageV1,
};
use a4n_db::shooting_script::ParsedChapterShootingScript;
use serde_json::Value;

pub(crate) fn parse_manga_page_plan(
    json: &Value,
    shooting_script: &ParsedChapterShootingScript,
    context: &str,
) -> Result<MangaPagePlanV3, AppError> {
    let plan: MangaPagePlanV3 = serde_json::from_value(json.clone()).map_err(|error| {
        AppError::InvalidArgument(format!("{context} 不符合 manga_page_plan_v3 合同: {error}"))
    })?;
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
