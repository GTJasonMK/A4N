use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::Value;

pub use a4n_character_host_contract::{
    CharacterProfileSyncPreviewStep, CharacterProfileSyncStageTraceInput,
    CharacterProfileSyncStageTracePort,
};

#[derive(Debug, Clone)]
pub struct CharacterProfileSyncStageInput {
    pub project_id: i64,
    pub workflow_run_id: Option<i64>,
    pub profile_id: i64,
    pub profile_name: String,
    pub start_chapter: i32,
    pub target_chapter: i32,
    pub selected_chapter_numbers: Vec<i32>,
    pub chapter_data: Vec<Value>,
    pub baseline_state: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterProfileSyncChapterStateDraft {
    pub chapter_number: i32,
    pub state_summary: String,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
    pub emotional_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterProfileSyncExtraction {
    pub changes: Vec<Value>,
    pub behaviors: Vec<Value>,
    pub deletion_candidates: Vec<Value>,
    pub chapter_states: Vec<CharacterProfileSyncChapterStateDraft>,
}

#[derive(Debug, Clone)]
pub struct CharacterProfileSyncGenerationTrace {
    pub request_json: Value,
    pub rendered_messages_json: Value,
    pub prompt_template_names_json: Value,
    pub response_chars: usize,
}

#[derive(Debug, Clone)]
pub struct CharacterProfileSyncStageOutput {
    pub extraction: CharacterProfileSyncExtraction,
    pub generation_trace: CharacterProfileSyncGenerationTrace,
    pub llm_trace_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CharacterProfileSyncStage {
    EvidenceEvents,
    AttributeChanges,
    ChapterStates,
    BehaviorRecords,
    DeletionCandidates,
}

impl CharacterProfileSyncStage {
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

    pub(super) fn temperature(self) -> f32 {
        match self {
            Self::EvidenceEvents => 0.2,
            Self::AttributeChanges | Self::ChapterStates | Self::BehaviorRecords => 0.25,
            Self::DeletionCandidates => 0.15,
        }
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

pub(super) struct CharacterProfileSyncStageExchange {
    pub(super) stage: CharacterProfileSyncStage,
    pub(super) request: ChatRequest,
    pub(super) response_content: String,
}

#[derive(Default)]
pub(super) struct CharacterProfileSyncStageOutputs {
    pub(super) evidence_events: Vec<Value>,
    pub(super) changes: Vec<Value>,
    pub(super) behaviors: Vec<Value>,
    pub(super) deletion_candidates: Vec<Value>,
    pub(super) chapter_states: Vec<CharacterProfileSyncChapterStateDraft>,
}
