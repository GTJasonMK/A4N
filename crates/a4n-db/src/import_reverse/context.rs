use crate::outline::{OutlineGenerationSourceRows, load_outline_generation_source_rows};
use sea_orm::ConnectionTrait;

use super::{ImportReverseDbError, ReverseBlueprintContextRows, ReverseContextRows};

pub async fn reverse_context_rows<C>(
    db: &C,
    project_id: i64,
    include_blueprint_context: bool,
) -> Result<ReverseContextRows, ImportReverseDbError>
where
    C: ConnectionTrait,
{
    let rows = load_outline_generation_source_rows(db, project_id)
        .await
        .map_err(ImportReverseDbError::Database)?
        .ok_or_else(|| {
            ImportReverseDbError::NotFound(format!("项目不存在: project_id={project_id}"))
        })?;
    let OutlineGenerationSourceRows {
        project,
        chapters,
        blueprint,
        blueprint_characters,
        blueprint_relationships,
        character_profiles,
        objects,
        scenes,
        foreshadowing,
        parts,
        chapter_outlines,
        chapter_outline_provenance: _,
    } = rows;
    let chapters = chapters
        .into_iter()
        .filter(|chapter| chapter.status != "deleted" && !chapter.content.trim().is_empty())
        .collect::<Vec<_>>();
    let blueprint_context = if include_blueprint_context {
        Some(ReverseBlueprintContextRows {
            foreshadowing,
            existing_blueprint: blueprint,
            blueprint_characters,
            blueprint_relationships,
            protagonist_profiles: character_profiles,
            objects,
            scenes,
        })
    } else {
        None
    };

    Ok(ReverseContextRows {
        project,
        chapters,
        chapter_outlines,
        part_outlines: parts,
        blueprint_context,
    })
}
