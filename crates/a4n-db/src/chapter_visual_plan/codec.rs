use a4n_db_models::models::chapter::chapter_visual_plan;
use a4n_domain::chapter_visual_plan::{
    CHAPTER_VISUAL_PLAN_V2, ChapterVisualPlanAuthoringMode, ChapterVisualPlanStatus,
    ChapterVisualPlanV2,
};

use super::contracts::{ChapterVisualPlanDbError, ParsedChapterVisualPlan};

pub(super) fn parse_record(
    record: chapter_visual_plan::Model,
) -> Result<ParsedChapterVisualPlan, ChapterVisualPlanDbError> {
    if record.contract_version != CHAPTER_VISUAL_PLAN_V2 {
        return Err(ChapterVisualPlanDbError::InvalidState(format!(
            "章节视觉编排 {} 使用不受支持的合同版本: {}",
            record.id, record.contract_version
        )));
    }
    if !valid_source_snapshot_identity(record.source_snapshot_id, record.source_snapshot_chapter_id)
    {
        return Err(ChapterVisualPlanDbError::InvalidState(format!(
            "章节视觉编排 {} 来源快照身份必须同时为空或同时为正数",
            record.id
        )));
    }
    validate_sha256(&record.source_content_sha256, "source_content_sha256")?;
    validate_sha256(&record.plan_sha256, "plan_sha256")?;
    let status = parse_status(&record.status)?;
    let authoring_mode = parse_authoring_mode(&record.authoring_mode)?;
    let plan: ChapterVisualPlanV2 =
        serde_json::from_value(record.plan_json.clone()).map_err(|error| {
            ChapterVisualPlanDbError::InvalidState(format!(
                "解析章节视觉编排 {} 失败: {error}",
                record.id
            ))
        })?;
    let validation = if status == ChapterVisualPlanStatus::Confirmed {
        plan.validate_confirmed()
    } else {
        plan.validate_draft()
    };
    validation.map_err(|error| {
        ChapterVisualPlanDbError::InvalidState(format!(
            "章节视觉编排 {} 校验失败: {error}",
            record.id
        ))
    })?;
    let actual = crate::generation::canonical_json_sha256(&plan).map_err(|error| {
        ChapterVisualPlanDbError::InvalidState(format!("计算章节视觉编排哈希失败: {error}"))
    })?;
    if actual != record.plan_sha256 {
        return Err(ChapterVisualPlanDbError::InvalidState(format!(
            "章节视觉编排 {} 内容哈希不一致",
            record.id
        )));
    }
    Ok(ParsedChapterVisualPlan {
        record,
        status,
        authoring_mode,
        plan,
    })
}

pub(super) fn serialize_plan(
    plan: &ChapterVisualPlanV2,
) -> Result<(serde_json::Value, String), ChapterVisualPlanDbError> {
    let plan_json = plan
        .canonical_json()
        .map_err(|error| ChapterVisualPlanDbError::InvalidArgument(error.to_string()))?;
    let plan_sha256 = crate::generation::canonical_json_sha256(plan).map_err(|error| {
        ChapterVisualPlanDbError::InvalidState(format!("计算章节视觉编排哈希失败: {error}"))
    })?;
    Ok((plan_json, plan_sha256))
}

fn parse_status(value: &str) -> Result<ChapterVisualPlanStatus, ChapterVisualPlanDbError> {
    match value {
        "draft" => Ok(ChapterVisualPlanStatus::Draft),
        "confirmed" => Ok(ChapterVisualPlanStatus::Confirmed),
        "superseded" => Ok(ChapterVisualPlanStatus::Superseded),
        "stale" => Ok(ChapterVisualPlanStatus::Stale),
        _ => Err(ChapterVisualPlanDbError::InvalidState(format!(
            "未知章节视觉编排状态: {value}"
        ))),
    }
}

fn parse_authoring_mode(
    value: &str,
) -> Result<ChapterVisualPlanAuthoringMode, ChapterVisualPlanDbError> {
    match value {
        "collaborative" => Ok(ChapterVisualPlanAuthoringMode::Collaborative),
        "manual" => Ok(ChapterVisualPlanAuthoringMode::Manual),
        _ => Err(ChapterVisualPlanDbError::InvalidState(format!(
            "未知章节视觉编排模式: {value}"
        ))),
    }
}

pub(super) fn validate_sha256(value: &str, label: &str) -> Result<(), ChapterVisualPlanDbError> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ChapterVisualPlanDbError::InvalidArgument(format!(
            "{label} 必须是 64 位十六进制 SHA-256"
        )))
    }
}

pub(super) fn valid_source_snapshot_identity(
    source_snapshot_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
) -> bool {
    match (source_snapshot_id, source_snapshot_chapter_id) {
        (None, None) => true,
        (Some(snapshot_id), Some(snapshot_chapter_id)) => {
            snapshot_id > 0 && snapshot_chapter_id > 0
        }
        _ => false,
    }
}
