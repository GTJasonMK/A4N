use super::*;

pub(in crate::project_dependency_graph) async fn manga_prompt_detail_records<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .order_by_asc(chapter_manga_prompt::Column::ChapterId)
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(prompts
        .into_iter()
        .map(|prompt| manga_prompt_record(prompt, None))
        .collect())
}

pub(in crate::project_dependency_graph) async fn manga_image_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    warnings: &mut Vec<String>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .order_by_asc(chapter_manga_prompt::Column::ChapterId)
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_image::Column::ImageType.eq("manga_page"))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let image_by_id = images
        .iter()
        .map(|image| (image.id, image))
        .collect::<BTreeMap<_, _>>();
    let mut linked_ids = BTreeSet::new();
    let mut records = Vec::new();
    for prompt in prompts {
        if let Some(image_id) = prompt.generated_image_id {
            linked_ids.insert(image_id);
            let image = image_by_id.get(&image_id).copied();
            let mut record = manga_prompt_record(prompt.clone(), image);
            record.id = format!("manga_image:{}", image_id);
            record.label = format!("第 {} 章 第 {} 页", prompt.chapter_id, prompt.page_number);
            record.asset_kind = "manga_page_image".to_string();
            record.asset_id = Some(image_id);
            record.origin = "image_generated".to_string();
            record.result = match image {
                Some(image) => db_record_result_with_related(
                    "generated_image",
                    image,
                    vec![db_related_record("chapter_manga_prompt", &prompt)],
                ),
                None => db_record_result("chapter_manga_prompt", &prompt),
            };
            if image.is_none() {
                record.warnings.push(format!(
                    "漫画提示词 #{} 绑定的图片 #{} 不存在。",
                    prompt.id, image_id
                ));
            }
            records.push(record);
        }
    }
    for image in images {
        if linked_ids.contains(&image.id) {
            continue;
        }
        warnings.push(format!(
            "漫画图片 #{} 没有 chapter_manga_prompt.generated_image_id 反链。",
            image.id
        ));
        let mut record = generated_image_record(image, "manga_page_image", "unknown");
        record
            .warnings
            .push("未找到对应漫画提示词来源。".to_string());
        records.push(record);
    }
    Ok(records)
}
