use super::*;

pub(super) async fn insert_import_text_project_part_outlines<C>(
    db: &C,
    project_id: i64,
    parts: &[ImportTextProjectPartOutline],
    now: NaiveDateTime,
) -> Result<HashMap<i64, i64>, ImportTextProjectError>
where
    C: ConnectionTrait,
{
    let mut id_map = HashMap::new();
    for part in parts {
        let inserted = part_outline::ActiveModel {
            project_id: Set(project_id),
            part_number: Set(part.part_number),
            title: Set(part.title.clone()),
            start_chapter: Set(part.start_chapter),
            end_chapter: Set(part.end_chapter),
            summary: Set(part.summary.clone()),
            theme: Set(part.theme.clone()),
            key_events: Set(part.key_events.clone()),
            character_focus: Set(part.character_focus.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|error| ImportTextProjectError::database("导入分部大纲失败", error))?;
        if let Some(old_id) = part.old_id {
            id_map.insert(old_id, inserted.id);
        }
    }
    Ok(id_map)
}

pub(super) async fn insert_import_text_project_chapter_outlines<C>(
    db: &C,
    project_id: i64,
    outlines: &[ImportTextProjectChapterOutline],
    part_id_map: &HashMap<i64, i64>,
    now: NaiveDateTime,
) -> Result<HashMap<i64, i64>, ImportTextProjectError>
where
    C: ConnectionTrait,
{
    let mut id_map = HashMap::new();
    for outline in outlines {
        let part_id = outline
            .old_part_id
            .map(|old_id| {
                part_id_map.get(&old_id).copied().ok_or_else(|| {
                    ImportTextProjectError::InvalidPackage(format!(
                        "第{}章大纲引用的分部大纲不存在: part_id={old_id}",
                        outline.chapter_number
                    ))
                })
            })
            .transpose()?;
        let inserted = chapter_outline::ActiveModel {
            project_id: Set(project_id),
            part_id: Set(part_id),
            chapter_number: Set(outline.chapter_number),
            title: Set(outline.title.clone()),
            summary: Set(outline.summary.clone()),
            beats: Set(outline.beats.clone()),
            key_scenes: Set(outline.key_scenes.clone()),
            pacing_score: Set(outline.pacing_score),
            foreshadowing_setup: Set(outline.foreshadowing_setup.clone()),
            foreshadowing_payoff: Set(outline.foreshadowing_payoff.clone()),
            characters_involved: Set(outline.characters_involved.clone()),
            word_count_target: Set(outline.word_count_target),
            status: Set(outline.status.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|error| ImportTextProjectError::database("导入章节大纲失败", error))?;
        if let Some(old_id) = outline.old_id {
            id_map.insert(old_id, inserted.id);
        }
    }
    Ok(id_map)
}
