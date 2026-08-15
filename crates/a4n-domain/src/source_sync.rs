use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

macro_rules! source_sync_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
        pub enum $name {
            $(#[serde(rename = $value)] $variant),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub const VALUES: &'static [&'static str] = &[$($value),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }

            pub fn parse(value: &str) -> Option<Self> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|candidate| candidate.as_str() == value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

source_sync_enum!(SourceSyncRunMode {
    DryRun => "dry_run",
    Apply => "apply",
});

source_sync_enum!(SourceSyncRunStatus {
    Planned => "planned",
    Applying => "applying",
    Completed => "completed",
});

source_sync_enum!(SourceSyncChangeKind {
    Added => "added",
    Updated => "updated",
    Reordered => "reordered",
    Deleted => "deleted",
    Moved => "moved",
    Split => "split",
});

source_sync_enum!(SourceSyncLocalStateKind {
    UserEdited => "user_edited",
    Generated => "generated",
    Imported => "imported",
    Assembled => "assembled",
    Clean => "clean",
    Draft => "draft",
});

source_sync_enum!(SourceSyncConflictSeverity {
    Blocking => "blocking",
    Warning => "warning",
    Info => "info",
});

source_sync_enum!(SourceSyncConflictStatus {
    Pending => "pending",
    Resolved => "resolved",
    Ignored => "ignored",
});

source_sync_enum!(SourceSyncResolutionStrategy {
    KeepLocal => "keep_local",
    AdoptSource => "adopt_source",
    RebuildPrompt => "rebuild_prompt",
    RebindAnchor => "rebind_anchor",
    RegenerateScene => "regenerate_scene",
    CreateReplacement => "create_replacement",
    Manual => "manual",
    Detach => "detach",
    DeleteLocal => "delete_local",
});

source_sync_enum!(SourceSyncDecisionActor {
    User => "user",
    System => "system",
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_sync_values_accept_only_canonical_spelling() {
        assert_eq!(
            SourceSyncResolutionStrategy::parse("regenerate_scene"),
            Some(SourceSyncResolutionStrategy::RegenerateScene)
        );
        assert!(SourceSyncResolutionStrategy::parse(" regenerate_scene ").is_none());
        assert!(!SourceSyncLocalStateKind::VALUES.contains(&"exported"));
    }
}
