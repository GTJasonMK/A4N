use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImportReviewStatus {
    Pending,
    Accepted,
    Rejected,
    Modified,
}

impl ImportReviewStatus {
    pub const ALL: [Self; 4] = [
        Self::Pending,
        Self::Accepted,
        Self::Rejected,
        Self::Modified,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Modified => "modified",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}
