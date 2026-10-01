use a4n_db_models::models::chapter::{chapter_outline, part_outline};
use a4n_domain::chapter::{ChapterOutlineRecord, PartOutlineRecord};

pub(crate) fn part_outline_record_from_model(model: part_outline::Model) -> PartOutlineRecord {
    PartOutlineRecord {
        id: model.id,
        project_id: model.project_id,
        part_number: model.part_number,
        title: model.title,
        start_chapter: model.start_chapter,
        end_chapter: model.end_chapter,
        summary: model.summary,
        theme: model.theme,
        key_events: model.key_events,
        character_focus: model.character_focus,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub(crate) fn part_outline_model_from_record(record: PartOutlineRecord) -> part_outline::Model {
    part_outline::Model {
        id: record.id,
        project_id: record.project_id,
        part_number: record.part_number,
        title: record.title,
        start_chapter: record.start_chapter,
        end_chapter: record.end_chapter,
        summary: record.summary,
        theme: record.theme,
        key_events: record.key_events,
        character_focus: record.character_focus,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

pub(crate) fn chapter_outline_record_from_model(
    model: chapter_outline::Model,
) -> ChapterOutlineRecord {
    ChapterOutlineRecord {
        id: model.id,
        project_id: model.project_id,
        part_id: model.part_id,
        chapter_number: model.chapter_number,
        title: model.title,
        summary: model.summary,
        beats: model.beats,
        key_scenes: model.key_scenes,
        pacing_score: model.pacing_score,
        foreshadowing_setup: model.foreshadowing_setup,
        foreshadowing_payoff: model.foreshadowing_payoff,
        characters_involved: model.characters_involved,
        word_count_target: model.word_count_target,
        status: model.status,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub(crate) fn chapter_outline_model_from_record(
    record: ChapterOutlineRecord,
) -> chapter_outline::Model {
    chapter_outline::Model {
        id: record.id,
        project_id: record.project_id,
        part_id: record.part_id,
        chapter_number: record.chapter_number,
        title: record.title,
        summary: record.summary,
        beats: record.beats,
        key_scenes: record.key_scenes,
        pacing_score: record.pacing_score,
        foreshadowing_setup: record.foreshadowing_setup,
        foreshadowing_payoff: record.foreshadowing_payoff,
        characters_involved: record.characters_involved,
        word_count_target: record.word_count_target,
        status: record.status,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;

    use super::*;

    #[test]
    fn part_outline_mapping_preserves_every_field() {
        let model = part_outline::Model {
            id: 1,
            project_id: 2,
            part_number: 3,
            title: "标题".to_string(),
            start_chapter: 4,
            end_chapter: 5,
            summary: Some("摘要".to_string()),
            theme: Some("主题".to_string()),
            key_events: Some(serde_json::json!(["事件"])),
            character_focus: Some(serde_json::json!({"角色": true})),
            created_at: NaiveDateTime::MIN,
            updated_at: NaiveDateTime::MAX,
        };

        let record = part_outline_record_from_model(model.clone());
        assert_eq!(
            serde_json::to_string(&record).unwrap(),
            serde_json::to_string(&model).unwrap()
        );
        assert_eq!(part_outline_model_from_record(record), model);
    }

    #[test]
    fn chapter_outline_mapping_preserves_every_field() {
        let model = chapter_outline::Model {
            id: 1,
            project_id: 2,
            part_id: Some(3),
            chapter_number: 4,
            title: Some("标题".to_string()),
            summary: Some("摘要".to_string()),
            beats: Some(serde_json::json!(["节拍"])),
            key_scenes: Some(serde_json::json!(["场景"])),
            pacing_score: Some(0.5),
            foreshadowing_setup: Some(serde_json::json!(["伏笔"])),
            foreshadowing_payoff: Some(serde_json::json!(["回收"])),
            characters_involved: Some(serde_json::json!(["角色"])),
            word_count_target: Some(1000),
            status: a4n_domain::chapter::chapter_outline_status::STATUS_COMPLETED.to_string(),
            created_at: NaiveDateTime::MIN,
            updated_at: NaiveDateTime::MAX,
        };

        let record = chapter_outline_record_from_model(model.clone());
        assert_eq!(
            serde_json::to_string(&record).unwrap(),
            serde_json::to_string(&model).unwrap()
        );
        assert_eq!(chapter_outline_model_from_record(record), model);
    }
}
