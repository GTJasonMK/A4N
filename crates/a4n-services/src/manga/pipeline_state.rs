use a4n_core::error::AppError;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MangaPipelineStartStage {
    EventBeats,
    CharacterBinding,
    SpeechSelection,
    VisualAnchorExtraction,
    ItemExtraction,
    ComicSynthesis,
    PagePlanning,
    DrawingPrompt,
}

impl MangaPipelineStartStage {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "event_beats" => Some(Self::EventBeats),
            "character_binding" => Some(Self::CharacterBinding),
            "speech_selection" => Some(Self::SpeechSelection),
            "visual_anchor_extraction" => Some(Self::VisualAnchorExtraction),
            "item_extraction" => Some(Self::ItemExtraction),
            "comic_synthesis" => Some(Self::ComicSynthesis),
            "page_planning" => Some(Self::PagePlanning),
            "drawing_prompt" => Some(Self::DrawingPrompt),
            _ => None,
        }
    }

    pub(super) fn status(self) -> (&'static str, i32, &'static str) {
        match self {
            Self::EventBeats => ("event_beats", 1, "正在抽取漫画事件卡片"),
            Self::CharacterBinding => ("character_binding", 2, "正在绑定出场角色身份与外观"),
            Self::SpeechSelection => ("speech_selection", 3, "正在选择关键对白、想法与旁白"),
            Self::VisualAnchorExtraction => {
                ("visual_anchor_extraction", 4, "正在提取场景和视觉锚点")
            }
            Self::ItemExtraction => ("item_extraction", 5, "正在抽取漫画关键物品"),
            Self::ComicSynthesis => ("comic_synthesis", 6, "正在综合漫画章节节奏"),
            Self::PagePlanning => ("page_planning", 7, "正在生成漫画分页规划"),
            Self::DrawingPrompt => ("drawing_prompt", 8, "正在生成整页漫画提示词"),
        }
    }
}

pub(super) fn should_restore_manga_stage_checkpoint(
    resume_from_checkpoint: bool,
    start_from_stage: Option<MangaPipelineStartStage>,
    stage_order: i32,
) -> bool {
    if let Some(start_stage) = start_from_stage {
        return stage_order < start_stage.status().1;
    }
    resume_from_checkpoint
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PipelineProgress {
    pub(crate) current: i32,
    pub(crate) total: i32,
}

pub(crate) fn pipeline_status_from_layout<'a>(
    layout: Option<&'a Value>,
    context: &str,
) -> Result<&'a str, AppError> {
    let layout = layout_object(layout, context)?;
    let status = layout
        .get("pipeline_status")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|status| !status.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{context}.pipeline_status 必须是非空字符串"))
        })?;
    Ok(status)
}

pub(crate) fn pipeline_progress_from_layout(
    layout: Option<&Value>,
    context: &str,
) -> Result<PipelineProgress, AppError> {
    let layout = layout_object(layout, context)?;
    let current = required_nonnegative_i32(layout, "current", context)?;
    let total = required_positive_i32(layout, "total", context)?;
    if current > total {
        return Err(AppError::InvalidArgument(format!(
            "{context}.current 不能大于 {context}.total"
        )));
    }
    Ok(PipelineProgress { current, total })
}

fn layout_object<'a>(
    layout: Option<&'a Value>,
    context: &str,
) -> Result<&'a Map<String, Value>, AppError> {
    layout
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.layout 必须是 JSON 对象")))
}

fn required_nonnegative_i32(
    layout: &Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<i32, AppError> {
    let value = required_i32(layout, key, context)?;
    if value < 0 {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 必须是非负整数"
        )));
    }
    Ok(value)
}

fn required_positive_i32(
    layout: &Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<i32, AppError> {
    let value = required_i32(layout, key, context)?;
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 必须是正整数"
        )));
    }
    Ok(value)
}

fn required_i32(layout: &Map<String, Value>, key: &str, context: &str) -> Result<i32, AppError> {
    let raw = layout
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是整数")))?;
    i32::try_from(raw)
        .map_err(|_| AppError::InvalidArgument(format!("{context}.{key} 超出 i32 可表示范围")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_pipeline_status_from_layout() {
        let layout = json!({"pipeline_status": " running "});

        assert_eq!(
            pipeline_status_from_layout(Some(&layout), "漫画流水线状态").unwrap(),
            "running"
        );
    }

    #[test]
    fn rejects_missing_pipeline_status() {
        let layout = json!({"status": "cancelled"});

        let err = pipeline_status_from_layout(Some(&layout), "漫画流水线状态").unwrap_err();

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("pipeline_status"));
    }

    #[test]
    fn reads_pipeline_progress_from_layout() {
        let layout = json!({"current": 3, "total": 8});

        assert_eq!(
            pipeline_progress_from_layout(Some(&layout), "漫画流水线状态").unwrap(),
            PipelineProgress {
                current: 3,
                total: 8
            }
        );
    }

    #[test]
    fn rejects_missing_pipeline_progress_counts() {
        let layout = json!({"current": 3});

        let err = pipeline_progress_from_layout(Some(&layout), "漫画流水线状态").unwrap_err();

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("total"));
    }

    #[test]
    fn rejects_current_greater_than_total() {
        let layout = json!({"current": 8, "total": 7});

        let err = pipeline_progress_from_layout(Some(&layout), "漫画流水线状态").unwrap_err();

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("current 不能大于"));
    }
}
