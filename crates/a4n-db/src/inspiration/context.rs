use super::contracts::{
    InspirationBlueprintRows, InspirationDbError, InspirationProjectContextRows,
    InspirationRecentChapterRow,
};
use super::project::ensure_inspiration_project_exists;
use a4n_core::models::{
    chapter::chapter,
    project::{blueprint_character, blueprint_relationship, novel_blueprint},
};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect, sea_query::Expr,
};

#[derive(Debug, FromQueryResult)]
struct InspirationRecentChapterQueryRow {
    chapter_number: i32,
    title: Option<String>,
    summary: Option<String>,
    analysis_data: Option<serde_json::Value>,
    word_count: i32,
    has_content: bool,
}

pub async fn inspiration_project_context_rows(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<InspirationProjectContextRows, InspirationDbError> {
    let project = ensure_inspiration_project_exists(db, project_id).await?;

    let blueprint_query = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db);
    let recent_chapters_query = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::ChapterNumber)
        .column(chapter::Column::Title)
        .column(chapter::Column::Summary)
        .column(chapter::Column::AnalysisData)
        .column(chapter::Column::WordCount)
        .column_as(
            Expr::cust(
                r#"content ~ E'[^[:space:]\u00A0\u1680\u2000-\u200A\u2028\u2029\u202F\u205F\u3000]'"#,
            ),
            "has_content",
        )
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_desc(chapter::Column::ChapterNumber)
        .limit(6)
        .into_model::<InspirationRecentChapterQueryRow>()
        .all(db);
    let (blueprint_result, recent_chapters_result) =
        futures::join!(blueprint_query, recent_chapters_query);
    let blueprint = match blueprint_result.map_err(InspirationDbError::Database)? {
        Some(blueprint) => {
            let characters_query = blueprint_character::Entity::find()
                .filter(blueprint_character::Column::ProjectId.eq(project_id))
                .all(db);
            let relationships_query = blueprint_relationship::Entity::find()
                .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                .all(db);
            let (characters_result, relationships_result) =
                futures::join!(characters_query, relationships_query);
            let characters = characters_result.map_err(InspirationDbError::Database)?;
            let relationships = relationships_result.map_err(InspirationDbError::Database)?;
            Some(InspirationBlueprintRows {
                blueprint,
                characters,
                relationships,
            })
        }
        None => None,
    };

    let mut recent_chapters = recent_chapters_result.map_err(InspirationDbError::Database)?;
    recent_chapters.reverse();
    let recent_chapters = recent_chapters
        .into_iter()
        .map(|row| InspirationRecentChapterRow {
            chapter_number: row.chapter_number,
            title: row.title,
            summary: row.summary,
            analysis_data: row.analysis_data,
            word_count: row.word_count,
            has_content: row.has_content,
        })
        .collect();

    Ok(InspirationProjectContextRows {
        project,
        blueprint,
        recent_chapters,
    })
}
