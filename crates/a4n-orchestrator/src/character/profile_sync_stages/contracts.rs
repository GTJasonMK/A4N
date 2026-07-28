use a4n_core::error::AppError;
use a4n_llm::provider::{ChatMessage, ChatRequest};
use serde_json::Value;

pub type CharacterProfileSyncStageProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Clone, Copy)]
pub struct CharacterProfileSyncStageProgress<'a> {
    pub reporter: Option<CharacterProfileSyncStageProgressReporter<'a>>,
}

impl CharacterProfileSyncStageProgress<'_> {
    pub(super) fn report(&self, current: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(current, CHARACTER_PROFILE_SYNC_PROGRESS_TOTAL, message)?;
        }
        Ok(())
    }
}

pub(super) const CHARACTER_PROFILE_SYNC_PROGRESS_TOTAL: u32 = 10;

#[derive(Debug, Clone)]
pub struct CharacterProfileSyncStageInput {
    pub project_id: i64,
    pub workflow_run_id: Option<i64>,
    pub profile_id: i64,
    pub profile_name: String,
    pub last_synced_chapter: i32,
    pub start_chapter: i32,
    pub target_chapter: i32,
    pub selected_chapter_numbers: Vec<i32>,
    pub chapter_data: Vec<Value>,
    pub baseline_state: Value,
    pub base_attributes: Value,
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

#[derive(Debug, Clone)]
pub struct CharacterProfileSyncPreviewStep {
    pub id: String,
    pub title: String,
    pub prompt_name: &'static str,
    pub status: &'static str,
    pub note: Option<String>,
    pub request: Option<ChatRequest>,
}

#[derive(Debug, Clone)]
pub struct CharacterProfileSyncStageTraceInput {
    pub profile_name: String,
    pub phase: String,
    pub messages: Vec<ChatMessage>,
    pub response_content: String,
    pub temperature: Option<f32>,
    pub parse_error: Option<String>,
}

pub trait CharacterProfileSyncStageTracePort: Send + Sync {
    fn record_stage_trace(&self, input: CharacterProfileSyncStageTraceInput) -> Option<String>;
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
