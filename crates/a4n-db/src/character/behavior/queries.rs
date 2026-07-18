use a4n_core::models::character::protagonist_behavior_record;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

pub async fn behavior_records_by_profile<C>(
    db: &C,
    profile_id: i64,
    chapter_from: Option<i32>,
    chapter_to: Option<i32>,
    limit: Option<u64>,
) -> Result<Vec<protagonist_behavior_record::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let has_chapter_range = chapter_from.is_some() || chapter_to.is_some();
    let mut query = protagonist_behavior_record::Entity::find()
        .filter(protagonist_behavior_record::Column::ProfileId.eq(profile_id));

    if let Some(from) = chapter_from {
        query = query.filter(protagonist_behavior_record::Column::ChapterNumber.gte(from));
    }
    if let Some(to) = chapter_to {
        query = query.filter(protagonist_behavior_record::Column::ChapterNumber.lte(to));
    }
    if let Some(limit) = limit {
        query = query.limit(limit);
    }
    let query = if limit.is_some() && !has_chapter_range {
        query
            .order_by_desc(protagonist_behavior_record::Column::ChapterNumber)
            .order_by_desc(protagonist_behavior_record::Column::Id)
    } else {
        query
            .order_by_asc(protagonist_behavior_record::Column::ChapterNumber)
            .order_by_asc(protagonist_behavior_record::Column::Id)
    };
    query.all(db).await
}

pub async fn implicit_review_behavior_records<C>(
    db: &C,
    profile_id: i64,
) -> Result<Vec<protagonist_behavior_record::Model>, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_behavior_record::Entity::find()
        .filter(protagonist_behavior_record::Column::ProfileId.eq(profile_id))
        .order_by_desc(protagonist_behavior_record::Column::ChapterNumber)
        .order_by_desc(protagonist_behavior_record::Column::Id)
        .all(db)
        .await
}
