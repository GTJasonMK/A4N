use a4n_core::models::project::novel_blueprint;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::{BlueprintDbError, NovelBlueprintUpsertInput};

/// 插入或更新项目蓝图主体。
pub async fn upsert_novel_blueprint_record<C>(
    db: &C,
    input: NovelBlueprintUpsertInput,
    existing: Option<novel_blueprint::Model>,
) -> Result<novel_blueprint::Model, BlueprintDbError>
where
    C: ConnectionTrait,
{
    if let Some(blueprint) = existing {
        let mut active: novel_blueprint::ActiveModel = blueprint.into();
        active.title = Set(input.title);
        active.genre = Set(input.genre);
        active.style = Set(Some(input.style));
        active.tone = Set(Some(input.tone));
        active.target_audience = Set(Some(input.target_audience));
        active.one_sentence_summary = Set(Some(input.one_sentence_summary));
        active.full_synopsis = Set(Some(input.full_synopsis));
        active.world_setting = Set(Some(input.world_setting));
        active.needs_part_outlines = Set(input.needs_part_outlines);
        active.total_chapters = Set(Some(input.total_chapters));
        active.chapters_per_part = Set(input.chapters_per_part);
        active.current_version = Set(input.current_version);
        active.updated_at = Set(input.now);
        Ok(active.update(db).await?)
    } else {
        Ok(novel_blueprint::ActiveModel {
            project_id: Set(input.project_id),
            title: Set(input.title),
            genre: Set(input.genre),
            style: Set(Some(input.style)),
            tone: Set(Some(input.tone)),
            target_audience: Set(Some(input.target_audience)),
            one_sentence_summary: Set(Some(input.one_sentence_summary)),
            full_synopsis: Set(Some(input.full_synopsis)),
            world_setting: Set(Some(input.world_setting)),
            current_version: Set(input.current_version),
            needs_part_outlines: Set(input.needs_part_outlines),
            total_chapters: Set(Some(input.total_chapters)),
            chapters_per_part: Set(input.chapters_per_part),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            created_at: Set(input.now),
            updated_at: Set(input.now),
            ..Default::default()
        }
        .insert(db)
        .await?)
    }
}
