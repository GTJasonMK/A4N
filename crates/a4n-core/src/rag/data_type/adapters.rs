use super::{RagDataType, rules};

impl RagDataType {
    pub const fn as_str(self) -> &'static str {
        rules::as_str(self)
    }

    pub const fn display_name(self) -> &'static str {
        rules::display_name(self)
    }

    pub const fn retrieval_weight(self) -> f32 {
        rules::retrieval_weight(self)
    }

    pub fn parse(input: &str) -> Option<Self> {
        rules::parse(input)
    }

    pub fn ingest_hint() -> String {
        rules::ingest_hint()
    }
}

impl std::fmt::Display for RagDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for RagDataType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&str> for RagDataType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value).ok_or(())
    }
}
