pub(crate) const MIN_PROVIDER_VISUAL_PROMPT_CHARS: usize = 100;
pub(crate) const MIN_PROVIDER_MOTION_PROMPT_CHARS: usize = 60;
pub(crate) const MIN_FRAME_PROMPT_CHARS: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProviderPromptTextViolation {
    Empty,
    TooShort {
        min_chars: usize,
        actual_chars: usize,
    },
    DictionaryLike,
    PreviewPlaceholder,
    InternalWorkflowTerm(&'static str),
    UploadStatusTerm(&'static str),
    StaleSection(&'static str),
    DuplicateMotionTimeline,
    MissingRequiredSection(&'static str),
    DirectPrompt(String),
}
