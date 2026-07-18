use a4n_core::error::AppError;
use a4n_db::character::{
    CharacterDeletionMarkRewindSummary, CharacterProfileDeletionSummary,
    CharacterProfileRollbackResult, CharacterTimelineDeletionSummary,
};

use super::{
    CharacterProfileRecordDeletionResult, CharacterRecordLifecycleError,
    CharacterTimelineRollbackLifecycleResult,
};
use crate::rag::{
    CharacterProfileRagDeletionSummary, CharacterTimelineRagDeletionSummary, RagSyncError,
};

#[test]
fn lifecycle_error_preserves_persistence_and_rag_messages() {
    let persistence = CharacterRecordLifecycleError::Persistence(AppError::NotFound(
        "角色档案不存在: 7".to_string(),
    ));
    assert_eq!(persistence.to_string(), "未找到：角色档案不存在: 7");

    let rag = CharacterRecordLifecycleError::Rag(RagSyncError::Delete {
        message: "删除角色属性事件向量失败".to_string(),
    });
    assert_eq!(rag.to_string(), "删除角色属性事件向量失败");
}

#[test]
fn profile_deletion_result_keeps_db_artifacts_and_rag_counts_together() {
    let result = CharacterProfileRecordDeletionResult {
        deletion: CharacterProfileDeletionSummary {
            project_id: 3,
            profile_id: 7,
            portrait_paths: vec!["portraits/7.png".to_string()],
            attribute_event_ids: vec![11, 12],
            state_index_ids: vec![21],
        },
        rag: CharacterProfileRagDeletionSummary {
            deleted_profile_rag: 2,
            deleted_state_rag: 1,
            deleted_attribute_event_rag: 2,
        },
    };

    assert_eq!(result.deletion.portrait_paths, ["portraits/7.png"]);
    assert_eq!(result.rag.deleted_profile_rag, 2);
    assert_eq!(result.rag.deleted_attribute_event_rag, 2);
}

#[test]
fn timeline_rollback_result_keeps_db_rewind_and_rag_counts_together() {
    let result = CharacterTimelineRollbackLifecycleResult {
        rollback: CharacterProfileRollbackResult {
            project_id: 3,
            deleted_timeline: CharacterTimelineDeletionSummary {
                attribute_event_ids: vec![11],
                state_index_ids: vec![21],
                deleted_attribute_events: 1,
                deleted_behaviors: 2,
                deleted_state_indexes: 1,
                deleted_snapshots: 1,
            },
            deletion_mark_rewind: CharacterDeletionMarkRewindSummary {
                deleted: 2,
                rewound: 1,
            },
            last_synced_chapter: 4,
        },
        rag: CharacterTimelineRagDeletionSummary {
            deleted_state_rag: 1,
            deleted_attribute_event_rag: 1,
        },
    };

    assert_eq!(result.rollback.deletion_mark_rewind.rewound, 1);
    assert_eq!(result.rollback.last_synced_chapter, 4);
    assert_eq!(result.rag.deleted_state_rag, 1);
}
