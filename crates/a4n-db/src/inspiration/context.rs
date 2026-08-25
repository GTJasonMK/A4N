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

    let blueprint = match novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(InspirationDbError::Database)?
    {
        Some(blueprint) => {
            let characters = blueprint_character::Entity::find()
                .filter(blueprint_character::Column::ProjectId.eq(project_id))
                .all(db)
                .await
                .map_err(InspirationDbError::Database)?;
            let relationships = blueprint_relationship::Entity::find()
                .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                .all(db)
                .await
                .map_err(InspirationDbError::Database)?;
            Some(InspirationBlueprintRows {
                blueprint,
                characters,
                relationships,
            })
        }
        None => None,
    };

    let mut recent_chapters = chapter::Entity::find()
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
        .all(db)
        .await
        .map_err(InspirationDbError::Database)?;
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
