use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_domain::manga::MangaPagePromptRecord;

pub(in crate::manga) fn manga_page_prompt_record_from_model(
    prompt: chapter_manga_prompt::Model,
) -> MangaPagePromptRecord {
    MangaPagePromptRecord {
        id: prompt.id,
        chapter_id: prompt.chapter_id,
        project_id: prompt.project_id,
        prompt_type: prompt.prompt_type,
        page_number: prompt.page_number,
        image_prompt: prompt.image_prompt,
        dialogue_text: prompt.dialogue_text,
        characters_in_frame: prompt.characters_in_frame,
        layout: prompt.layout,
        generated_image_id: prompt.generated_image_id,
        status: prompt.status,
        created_at: prompt.created_at,
        updated_at: prompt.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db_models::models::chapter::chapter_manga_prompt::MANGA_PAGE_PROMPT_TYPE;
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn mapping_preserves_page_prompt_read_fields() {
        let created_at = NaiveDate::from_ymd_opt(2026, 9, 27)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let updated_at = created_at + chrono::Duration::minutes(1);
        let prompt = chapter_manga_prompt::Model {
            id: 1,
            chapter_id: 2,
            project_id: 3,
            page_number: 4,
            prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
            image_prompt: "page prompt".to_string(),
            dialogue_text: Some("dialogue".to_string()),
            characters_in_frame: Some(json!(["hero"])),
            layout: Some(json!({"schema_version": "manga_page_generation_v4"})),
            generated_image_id: Some(5),
            status: "cancelled".to_string(),
            created_at,
            updated_at,
        };

        assert_eq!(
            manga_page_prompt_record_from_model(prompt),
            MangaPagePromptRecord {
                id: 1,
                chapter_id: 2,
                project_id: 3,
                prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
                page_number: 4,
                image_prompt: "page prompt".to_string(),
                dialogue_text: Some("dialogue".to_string()),
                characters_in_frame: Some(json!(["hero"])),
                layout: Some(json!({"schema_version": "manga_page_generation_v4"})),
                generated_image_id: Some(5),
                status: "cancelled".to_string(),
                created_at,
                updated_at,
            }
        );
    }
}
