use a4n_core::import_reverse::ReverseContextKind;
use a4n_domain::project::ProjectStatus;

pub use a4n_api_contract::import_reverse::ReverseEngineeringTarget;

pub(crate) trait ReverseEngineeringTargetDomain {
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
