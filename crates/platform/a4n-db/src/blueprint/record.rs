use a4n_db_models::models::project::novel_blueprint;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::{BlueprintDbError, ManualBlueprintUpsertInput, NovelBlueprintUpsertInput};

pub fn blueprint_record(
    blueprint: &novel_blueprint::Model,
) -> a4n_domain::project::BlueprintRecord {
    a4n_domain::project::BlueprintRecord {
        id: blueprint.id,
        project_id: blueprint.project_id,
        title: blueprint.title.clone(),
        current_version: blueprint.current_version.clone(),
        total_chapters: blueprint.total_chapters,
        chapters_per_part: blueprint.chapters_per_part,
    }
}

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

/// 用户手工保存只覆盖表单字段，世界观、头像、角色和关系由各自资料入口维护。
pub async fn upsert_manual_blueprint_record<C>(
    db: &C,
    input: ManualBlueprintUpsertInput,
    existing: Option<novel_blueprint::Model>,
) -> Result<novel_blueprint::Model, BlueprintDbError>
where
    C: ConnectionTrait,
{
    if let Some(blueprint) = existing {
        let mut active: novel_blueprint::ActiveModel = blueprint.into();
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
        return Ok(active.update(db).await?);
    }

    Ok(novel_blueprint::ActiveModel {
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
    .await?)
}
