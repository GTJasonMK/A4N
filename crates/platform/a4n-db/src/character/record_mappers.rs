use a4n_db_models::models::character::{
    attribute_event, character_state_index, protagonist_behavior_record, protagonist_deletion_mark,
    snapshot_reference,
};
use a4n_domain::character::{
    CharacterAttributeEventRecord, CharacterBehaviorRecord, CharacterDeletionMarkRecord,
    CharacterSnapshotReferenceRecord, CharacterStateIndexRecord,
};

pub(in crate::character) fn character_attribute_event_record(
    model: attribute_event::Model,
) -> CharacterAttributeEventRecord {
    CharacterAttributeEventRecord {
        id: model.id,
        profile_id: model.profile_id,
        chapter_number: model.chapter_number,
        source_kind: model.source_kind,
        timestamp: model.timestamp,
        category: model.category,
        key: model.key,
        operation: model.operation,
        change_description: model.change_description,
        old_value: model.old_value,
        new_value: model.new_value,
        cause: model.cause,
        evidence: model.evidence,
    }
}

pub(in crate::character) fn character_behavior_record(
    model: protagonist_behavior_record::Model,
) -> CharacterBehaviorRecord {
    CharacterBehaviorRecord {
        id: model.id,
        profile_id: model.profile_id,
        chapter_number: model.chapter_number,
        source_kind: model.source_kind,
        behavior_description: model.behavior_description,
        behavior_type: model.behavior_type,
        original_text: model.original_text,
        behavior_tags: model.behavior_tags,
        classification_results: model.classification_results,
        related_characters: model.related_characters,
        impact_analysis: model.impact_analysis,
        created_at: model.created_at,
    }
}

pub(in crate::character) fn character_deletion_mark_record(
    model: protagonist_deletion_mark::Model,
) -> CharacterDeletionMarkRecord {
    CharacterDeletionMarkRecord {
        id: model.id,
        profile_id: model.profile_id,
        source_kind: model.source_kind,
        category: model.category,
        key: model.key,
        chapter_number: model.chapter_number,
        reason: model.reason,
        evidence: model.evidence,
        consecutive_count: model.consecutive_count,
        is_executed: model.is_executed,
        last_marked_chapter: model.last_marked_chapter,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub(in crate::character) fn character_state_index_record(
    model: character_state_index::Model,
) -> CharacterStateIndexRecord {
    CharacterStateIndexRecord {
        id: model.id,
        profile_id: model.profile_id,
        chapter_number: model.chapter_number,
        state_summary: model.state_summary,
        key_attributes: model.key_attributes,
        emotional_state: model.emotional_state,
        current_location: model.current_location,
        current_goal: model.current_goal,
        created_at: model.created_at,
    }
}

pub(in crate::character) fn character_snapshot_reference_record(
    model: snapshot_reference::Model,
) -> CharacterSnapshotReferenceRecord {
    CharacterSnapshotReferenceRecord {
        id: model.id,
        profile_id: model.profile_id,
        chapter_number: model.chapter_number,
        snapshot_hash: model.snapshot_hash,
        reference_count: model.reference_count,
        data: model.data,
        created_at: model.created_at,
    }
}
