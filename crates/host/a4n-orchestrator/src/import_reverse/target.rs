use super::context::ReverseContextKind;

pub(crate) use a4n_api_contract::import_reverse::ReverseEngineeringTarget;

pub(super) trait ReverseEngineeringTargetDomain {
    fn context_kind(self) -> ReverseContextKind;
}

impl ReverseEngineeringTargetDomain for ReverseEngineeringTarget {
    fn context_kind(self) -> ReverseContextKind {
        match self {
            Self::Blueprint => ReverseContextKind::Blueprint,
            Self::Outline => ReverseContextKind::Outline,
            Self::PartOutline => ReverseContextKind::PartOutline,
        }
    }
}
