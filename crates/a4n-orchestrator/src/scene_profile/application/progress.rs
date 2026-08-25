use super::types::SceneProfileStageProgressSpec;
use a4n_core::structured_progress::{FieldMilestone, ProgressSpan, StructuredJsonProgressSpec};
use tokio_util::sync::CancellationToken;

pub const SCENE_PROFILE_SYNC_PROGRESS_TOTAL: u32 = 100;
pub(super) const SCENE_PROFILE_SYNC_CHAPTER_START: u32 = 5;
pub(super) const SCENE_PROFILE_SYNC_CHAPTER_END: u32 = 88;
pub(super) const SCENE_PROFILE_SYNC_RAG_PROGRESS: u32 = 94;

pub(super) const SCENE_CANDIDATE_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("scene_ref", "场景引用", 1),
    FieldMilestone::new("name", "场景名称", 1),
    FieldMilestone::new("scene_type", "场景类型", 1),
    FieldMilestone::new("importance", "重要性", 1),
    FieldMilestone::new("base_description", "基础描述", 1),
    FieldMilestone::new("visual_description", "视觉描述", 1),
    FieldMilestone::new("spatial_layout", "空间布局", 1),
    FieldMilestone::new("atmosphere", "氛围", 1),
    FieldMilestone::new("evidence_anchor_ids", "证据锚点", 1),
];
pub(super) const SCENE_OCCURRENCE_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("occurrence_ref", "出场引用", 1),
    FieldMilestone::new("scene_ref", "关联场景", 1),
    FieldMilestone::new("summary", "出场摘要", 1),
    FieldMilestone::new("characters", "出场角色", 1),
    FieldMilestone::new("objects", "出场物体", 1),
    FieldMilestone::new("entry_state", "进入状态", 1),
    FieldMilestone::new("exit_state", "离开状态", 1),
    FieldMilestone::new("source_anchor_ids", "来源锚点", 1),
];
pub(super) const SCENE_TRANSITION_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("from_scene_ref", "起点场景", 1),
    FieldMilestone::new("to_scene_ref", "目标场景", 1),
    FieldMilestone::new("movement_type", "移动类型", 1),
    FieldMilestone::new("route_description", "路线描述", 1),
    FieldMilestone::new("participants", "参与角色", 1),
    FieldMilestone::new("object_refs", "关联物体", 1),
    FieldMilestone::new("source_anchor_ids", "来源锚点", 1),
];
pub(super) const SCENE_ENTITY_STATE_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("scene_ref", "关联场景", 1),
    FieldMilestone::new("occurrence_ref", "关联出场", 1),
    FieldMilestone::new("entity_type", "实体类型", 1),
    FieldMilestone::new("entity_id", "实体 ID", 1),
    FieldMilestone::new("state_key", "状态类型", 1),
    FieldMilestone::new("state_before", "变化前状态", 1),
    FieldMilestone::new("state_after", "变化后状态", 1),
    FieldMilestone::new("action_description", "动作描述", 1),
    FieldMilestone::new("source_anchor_ids", "来源锚点", 1),
];

type SceneProfileSyncProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

/// 活体信号出口：把模型输出的原始尾巴交给 Host，与字段级里程碑取自同一个 token 流。
type SceneProfileSyncStreamReporter<'a> = &'a (dyn Fn(&str) + Send + Sync);

pub struct SceneProfileSyncProgress<'a> {
    reporter: Option<SceneProfileSyncProgressReporter<'a>>,
    stream_reporter: Option<SceneProfileSyncStreamReporter<'a>>,
    pub(super) cancel_token: Option<&'a CancellationToken>,
    pub(super) active_task_id: Option<&'a str>,
}

impl<'a> SceneProfileSyncProgress<'a> {
    pub fn new(
        reporter: Option<SceneProfileSyncProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
        active_task_id: Option<&'a str>,
    ) -> Self {
        Self {
            reporter,
            stream_reporter: None,
            cancel_token,
            active_task_id,
        }
    }

    /// 附加活体信号出口。不提供时行为与之前完全一致。
    pub fn with_stream_reporter(
        mut self,
        stream_reporter: Option<SceneProfileSyncStreamReporter<'a>>,
    ) -> Self {
        self.stream_reporter = stream_reporter;
        self
    }

    pub(super) fn stream_text(&self, text: &str) {
        if let Some(reporter) = self.stream_reporter {
            reporter(text);
        }
    }

    pub(super) fn has_stream_reporter(&self) -> bool {
        self.stream_reporter.is_some()
    }

    fn progress(&self, current: u32, total: u32, message: &str) {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
    }

    fn check_cancelled(&self, message: &str) -> Result<(), String> {
        if self.cancel_token.is_some_and(|token| token.is_cancelled()) {
            Err(message.to_string())
        } else {
            Ok(())
        }
    }
}

pub(super) fn subspan(
    span: ProgressSpan,
    start_units: u32,
    end_units: u32,
    total_units: u32,
) -> ProgressSpan {
    let total_units = total_units.max(1);
    let start_units = start_units.min(total_units);
    let end_units = end_units.min(total_units).max(start_units);
    let width = span.end() - span.start();
    let mapped_start = u64::from(width) * u64::from(start_units) / u64::from(total_units);
    let mapped_end = u64::from(width) * u64::from(end_units) / u64::from(total_units);
    ProgressSpan::new(
        span.start()
            + u32::try_from(mapped_start)
                .expect("scene subspan start is bounded by the u32 span width"),
        span.start()
            + u32::try_from(mapped_end)
                .expect("scene subspan end is bounded by the u32 span width"),
    )
}

pub(super) fn scene_stage_progress_spec(
    root_array_key: &'static str,
    item_label: &'static str,
    milestones: &'static [FieldMilestone],
    span: ProgressSpan,
) -> SceneProfileStageProgressSpec {
    SceneProfileStageProgressSpec {
        structured: StructuredJsonProgressSpec {
            root_array_key,
            item_label,
            expected_items: None,
            milestones,
        },
        span,
    }
}

pub(super) fn report_scene_profile_sync_progress(
    progress: Option<&SceneProfileSyncProgress<'_>>,
    current: u32,
    message: impl AsRef<str>,
    cancelled_message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.progress(current, SCENE_PROFILE_SYNC_PROGRESS_TOTAL, message.as_ref());
        progress.check_cancelled(cancelled_message)?;
    }
    Ok(())
}
