use a4n_db_models::models::project::novel_blueprint;
use a4n_domain::project::BlueprintRecord;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::{BlueprintDbError, ManualBlueprintUpsertInput, NovelBlueprintUpsertInput};

/// 插入或更新项目蓝图主体。
pub async fn upsert_novel_blueprint_record<C>(
    db: &C,
    input: NovelBlueprintUpsertInput,
    existing: Option<BlueprintRecord>,
) -> Result<BlueprintRecord, BlueprintDbError>
where
    C: ConnectionTrait,
{
    if let Some(blueprint) = existing {
        let mut active: novel_blueprint::ActiveModel =
            crate::project::blueprint_model_from_record(blueprint).into();
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
        Ok(crate::project::blueprint_record(active.update(db).await?))
    } else {
        let model = novel_blueprint::ActiveModel {
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
        .await?;
        Ok(crate::project::blueprint_record(model))
    }
}

/// 用户手工保存只覆盖表单字段，世界观、头像、角色和关系由各自资料入口维护。
pub async fn upsert_manual_blueprint_record<C>(
    db: &C,
    input: ManualBlueprintUpsertInput,
    existing: Option<BlueprintRecord>,
) -> Result<BlueprintRecord, BlueprintDbError>
where
    C: ConnectionTrait,
{
    if let Some(blueprint) = existing {
        let mut active: novel_blueprint::ActiveModel =
            crate::project::blueprint_model_from_record(blueprint).into();
        active.title = Set(input.title);
        active.genre = Set(input.genre);
        active.style = Set(input.style);
        active.tone = Set(input.tone);
        active.target_audience = Set(input.target_audience);
        active.one_sentence_summary = Set(input.one_sentence_summary);
        active.full_synopsis = Set(input.full_synopsis);
        active.needs_part_outlines = Set(input.needs_part_outlines);
        active.total_chapters = Set(Some(input.total_chapters));
        active.chapters_per_part = Set(input.chapters_per_part);
        active.current_version = Set(input.current_version);
        active.updated_at = Set(input.now);
        return Ok(crate::project::blueprint_record(active.update(db).await?));
    }

    let model = novel_blueprint::ActiveModel {
        project_id: Set(input.project_id),
        title: Set(input.title),
        genre: Set(input.genre),
        style: Set(input.style),
        tone: Set(input.tone),
        target_audience: Set(input.target_audience),
        one_sentence_summary: Set(input.one_sentence_summary),
        full_synopsis: Set(input.full_synopsis),
        world_setting: Set(None),
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
    .await?;
    Ok(crate::project::blueprint_record(model))
}
