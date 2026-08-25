use a4n_domain::project::ProjectStatus;

use super::context::ReverseContextKind;

pub(crate) use a4n_api_contract::import_reverse::ReverseEngineeringTarget;

pub(super) trait ReverseEngineeringTargetDomain {
    fn context_kind(self) -> ReverseContextKind;
    fn applied_status(self) -> ProjectStatus;
}

impl ReverseEngineeringTargetDomain for ReverseEngineeringTarget {
    fn context_kind(self) -> ReverseContextKind {
        match self {
            Self::Blueprint => ReverseContextKind::Blueprint,
            Self::Outline => ReverseContextKind::Outline,
            Self::PartOutline => ReverseContextKind::PartOutline,
        }
    }

    fn applied_status(self) -> ProjectStatus {
        match self {
            Self::Blueprint => ProjectStatus::BlueprintReady,
            Self::Outline => ProjectStatus::ChapterOutlinesReady,
            Self::PartOutline => ProjectStatus::PartOutlinesReady,
        }
    }
}
