//! Pure text-anchor evidence values.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchorProfileKind {
    Character,
    Object,
    Scene,
}

impl TextAnchorProfileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Object => "object",
            Self::Scene => "scene",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextAnchorEvidenceInput {
    pub text_anchor_id: i64,
    pub evidence_role: String,
}
