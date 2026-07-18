use super::prelude::*;

pub(in crate::actions::character_ai) struct CharacterSyncGenerationTrace {
    pub(super) request_json: Value,
    pub(super) rendered_messages_json: Value,
    pub(super) prompt_template_names_json: Value,
    pub(super) response_chars: usize,
}

impl CharacterSyncGenerationTrace {
    pub(super) fn from_stage_exchanges(
        exchanges: &[CharacterSyncStageExchange],
    ) -> Result<Self, String> {
        let stage_requests = exchanges
            .iter()
            .map(|exchange| {
                context_aware_chat_request_trace_json(&exchange.request).map(|request| {
                    json!({
                        "prompt_name": exchange.stage.prompt_name(),
                        "request": request})
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("角色同步分阶段 LLM 请求序列化失败: {err}"))?;
        let stage_messages = exchanges
            .iter()
            .map(|exchange| {
                serde_json::to_value(&exchange.request.messages).map(|messages| {
                    json!({
                        "prompt_name": exchange.stage.prompt_name(),
                        "messages": messages})
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("角色同步分阶段 LLM 消息序列化失败: {err}"))?;
        Ok(Self {
            request_json: json!({ "stages": stage_requests }),
            rendered_messages_json: json!({ "stages": stage_messages }),
            prompt_template_names_json: json!(
                exchanges
                    .iter()
                    .map(|exchange| exchange.stage.prompt_name())
                    .collect::<Vec<_>>()
            ),
            response_chars: exchanges
                .iter()
                .map(|exchange| exchange.response_content.chars().count())
                .sum(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(in crate::actions::character_ai) enum CharacterSyncStage {
    EvidenceEvents,
    AttributeChanges,
    ChapterStates,
    BehaviorRecords,
    DeletionCandidates,
}

impl CharacterSyncStage {
    pub(super) const ALL: [Self; 5] = [
        Self::EvidenceEvents,
        Self::AttributeChanges,
        Self::ChapterStates,
        Self::BehaviorRecords,
        Self::DeletionCandidates,
    ];

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::EvidenceEvents => "character_sync_evidence_events",
            Self::AttributeChanges => "character_sync_attribute_changes",
            Self::ChapterStates => "character_sync_chapter_states",
            Self::BehaviorRecords => "character_sync_behavior_records",
            Self::DeletionCandidates => "character_sync_deletion_candidates",
        }
    }

    pub(super) fn prompt_name(self) -> &'static str {
        self.key()
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::EvidenceEvents => "角色同步 Step 1：证据事件抽取",
            Self::AttributeChanges => "角色同步 Step 2：属性变化判断",
            Self::ChapterStates => "角色同步 Step 3：章节状态生成",
            Self::BehaviorRecords => "角色同步 Step 4：关键行为记录",
            Self::DeletionCandidates => "角色同步 Step 5：删除候选判断",
        }
    }

    pub(super) fn progress_message(self, name: &str) -> String {
        match self {
            Self::EvidenceEvents => format!("{name}：正在抽取角色相关证据事件"),
            Self::AttributeChanges => format!("{name}：正在判断角色属性变化"),
            Self::ChapterStates => format!("{name}：正在生成章节状态"),
            Self::BehaviorRecords => format!("{name}：正在记录关键行为"),
            Self::DeletionCandidates => format!("{name}：正在判断删除候选"),
        }
    }

    pub(super) fn root_field(self) -> &'static str {
        match self {
            Self::EvidenceEvents => "events",
            Self::AttributeChanges => "attribute_changes",
            Self::ChapterStates => "chapter_states",
            Self::BehaviorRecords => "behaviors",
            Self::DeletionCandidates => "deletion_candidates",
        }
    }
}

pub(super) struct CharacterSyncStageExchange {
    pub(super) stage: CharacterSyncStage,
    pub(super) request: ChatRequest,
    pub(super) response_content: String,
}

pub(super) struct CharacterSyncStageRun {
    pub(super) extraction: CharacterSyncExtraction,
    pub(super) generation_trace: CharacterSyncGenerationTrace,
    pub(super) llm_trace_paths: Vec<String>,
}

#[derive(Default)]
pub(super) struct CharacterSyncStageOutputs {
    pub(super) evidence_events: Vec<Value>,
    pub(super) changes: Vec<Value>,
    pub(super) behaviors: Vec<Value>,
    pub(super) deletion_candidates: Vec<Value>,
    pub(super) chapter_states: Vec<ChapterStateDraft>,
}
