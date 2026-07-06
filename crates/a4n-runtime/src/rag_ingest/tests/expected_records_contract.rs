use super::super::chapters::chapter_should_sync_rag_records;
use super::super::expected_records::ExpectedRagRecordCounts;
use super::sample_chapter;

#[test]
fn chapter_expected_rag_records_use_same_visibility_rule_as_ingest() {
    assert!(chapter_should_sync_rag_records(&sample_chapter(
        "completed",
        "旧城广播忽然响起。"
    )));
    assert!(!chapter_should_sync_rag_records(&sample_chapter(
        "completed",
        " \n\t "
    )));
    assert!(!chapter_should_sync_rag_records(&sample_chapter(
        "deleted",
        "这章仍有旧正文，但已被删除。"
    )));
}

#[test]
fn expected_rag_record_counts_total_sums_record_counts() {
    let counts = ExpectedRagRecordCounts {
        chapters: 3,
        outlines: 1,
        part_outlines: 2,
        blueprints: 1,
        blueprint_metadata: 1,
        world_settings: 2,
        characters: 7,
        protagonists: 8,
        character_states: 1,
        character_attribute_events: 4,
        foreshadowing: 5,
        relationships: 6,
        scene_profiles: 2,
        scene_occurrences: 3,
        scene_transitions: 4,
    };

    assert_eq!(counts.total(), 50);
}
