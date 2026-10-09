use a4n_db_models::models::{
    ai::generated_image,
    chapter::{chapter_manga_prompt, chapter_shooting_script, manga_pipeline_artifact},
    character::character_portrait,
    media::media_asset::FILE_PATH_FIELD,
};
use a4n_domain::{
    CanvasNodeRefKind,
    manga::{MangaPageGenerationLayoutV3, MangaPageImageArtifactV1, MangaPipelineArtifactStage},
    source_text_hash,
};
use sea_orm::sea_query::{Alias, CaseStatement, Expr};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};

use crate::project::family::import::contracts::{
    PROJECT_FAMILY_CHARACTER_PORTRAIT_IMAGE_PATH_FIELD,
    PROJECT_FAMILY_MEDIA_OWNER_CHARACTER_PORTRAIT,
};
use crate::project::family::import::{
    DatabaseProjectFamilyImportPackage, ImportIdMaps, ProjectFamilyImportError,
    ProjectFamilyMediaFileImporter, map_optional, map_required, remap_chapter_source_revision_key,
    remap_manga_generated_image_metadata, remap_manga_page_layout,
    remap_manga_pipeline_artifact_payload, remap_shooting_script_visual_plan_source,
    remap_visual_plan_identity,
};

pub(super) async fn insert_manga_rows<Importer>(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
    media_importer: &mut Importer,
) -> Result<(), ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    insert_shooting_scripts(txn, package, maps).await?;
    insert_character_portraits(txn, package, maps, media_importer).await?;
    insert_generated_images(txn, package, maps, media_importer).await?;
    super::visual_plan::remap_visual_plan_resources(txn, package, maps).await?;
    super::authoring::remap_authoring_documents(txn, package, maps).await?;
    insert_manga_prompts(txn, package, maps).await?;
    insert_manga_pipeline_artifacts(txn, package, maps).await?;
    update_generated_image_metadata(txn, package, maps).await
}

async fn insert_shooting_scripts(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_mapped_import_rows!(
        txn,
        &package.chapter_shooting_scripts,
        script,
        chapter_shooting_script::ActiveModel,
        maps.shooting_script_ids,
        "导入章节拍摄剧本失败",
        |active| {
            maps.shooting_script_sources
                .insert(script.id, script.into());
            let source_chapter_id = map_required(
                &maps.chapter_ids,
                script.source_chapter_id,
                "章节拍摄剧本 source_chapter_id",
            )?;
            let source_snapshot_id = map_optional(
                &maps.source_snapshot_ids,
                script.source_snapshot_id,
                "章节拍摄剧本 source_snapshot_id",
            )?;
            let source_snapshot_chapter_id = map_optional(
                &maps.source_snapshot_chapter_ids,
                script.source_snapshot_chapter_id,
                "章节拍摄剧本 source_snapshot_chapter_id",
            )?;
            let (visual_plan_id, visual_plan_version, visual_plan_sha256) =
                remap_visual_plan_identity(
                    script.visual_plan_id,
                    script.visual_plan_version,
                    script.visual_plan_sha256.as_deref(),
                    maps,
                    "章节拍摄剧本 visual plan",
                )?;
            let (script_json, script_sha256) = remap_shooting_script_visual_plan_source(
                &script.script_json,
                (
                    script.visual_plan_id,
                    script.visual_plan_version,
                    script.visual_plan_sha256.as_deref(),
                ),
                (
                    visual_plan_id,
                    visual_plan_version,
                    visual_plan_sha256.as_deref(),
                ),
                &format!("章节拍摄剧本 #{}", script.id),
            )?;
            let source_revision_key = remap_chapter_source_revision_key(
                &format!("章节拍摄剧本 #{}", script.id),
                (
                    script.source_chapter_id,
                    script.source_snapshot_id,
                    script.source_snapshot_chapter_id,
                ),
                (
                    source_chapter_id,
                    source_snapshot_id,
                    source_snapshot_chapter_id,
                ),
                &script.source_content_sha256,
                &script.source_revision_key,
            )?;
            maps.shooting_script_revision_keys
                .insert(script.id, source_revision_key.clone());
            maps.shooting_script_sha256s
                .insert(script.id, script_sha256.clone());
            active.project_id = Set(map_required(
                &maps.project_ids,
                script.project_id,
                "拍摄剧本所属视频项目",
            )?);
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                script.root_project_id,
                "章节拍摄剧本 root_project_id",
            )?);
            active.source_chapter_id = Set(source_chapter_id);
            active.source_snapshot_id = Set(source_snapshot_id);
            active.source_snapshot_chapter_id = Set(source_snapshot_chapter_id);
            active.source_revision_key = Set(source_revision_key);
            active.visual_plan_id = Set(visual_plan_id);
            active.visual_plan_version = Set(visual_plan_version);
            active.visual_plan_sha256 = Set(visual_plan_sha256);
            active.script_json = Set(script_json);
            active.script_sha256 = Set(script_sha256);
            active.workflow_run_id = Set(None);
        }
    );
    Ok(())
}

async fn insert_character_portraits<Importer>(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
    media_importer: &mut Importer,
) -> Result<(), ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    if package.character_portraits.is_empty() {
        return Ok(());
    }
    let rows = package
        .character_portraits
        .iter()
        .map(|portrait| {
            let mut active: character_portrait::ActiveModel = portrait.clone().into();
            active.id = sea_orm::NotSet;
            active.project_id = Set(map_required(
                &maps.project_ids,
                portrait.project_id,
                "角色立绘 project_id",
            )?);
            active.profile_id = Set(map_required(
                &maps.character_profile_ids,
                portrait.profile_id,
                "角色立绘 profile_id",
            )?);
            active.image_path = Set(String::new());
            Ok::<_, ProjectFamilyImportError>(active)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let inserted = character_portrait::Entity::insert_many(rows)
        .exec_with_returning_many(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "导入角色立绘失败",
            source,
        })?;
    if inserted.len() != package.character_portraits.len() {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "导入角色立绘批量写入返回行数不一致: inserted={}, expected={}",
            inserted.len(),
            package.character_portraits.len()
        )));
    }
    let mut image_path_case = CaseStatement::new();
    let mut portrait_ids = Vec::with_capacity(inserted.len());
    for (portrait, inserted) in package.character_portraits.iter().zip(inserted) {
        let image_path = media_importer
            .write_imported_media_field(
                inserted.project_id,
                PROJECT_FAMILY_MEDIA_OWNER_CHARACTER_PORTRAIT,
                portrait.id,
                inserted.id,
                PROJECT_FAMILY_CHARACTER_PORTRAIT_IMAGE_PATH_FIELD,
            )?
            .ok_or_else(|| {
                ProjectFamilyImportError::invalid_package(format!(
                    "角色立绘缺少媒体文件: {}",
                    portrait.id
                ))
            })?;
        image_path_case = image_path_case.case(
            character_portrait::Column::Id.eq(inserted.id),
            Expr::value(image_path.clone()),
        );
        portrait_ids.push(inserted.id);
        maps.character_portrait_ids.insert(portrait.id, inserted.id);
        maps.character_portrait_paths
            .insert(portrait.id, image_path);
    }
    let updated = character_portrait::Entity::update_many()
        .col_expr(
            character_portrait::Column::ImagePath,
            image_path_case
                .finally(Expr::col(character_portrait::Column::ImagePath))
                .into(),
        )
        .filter(character_portrait::Column::Id.is_in(portrait_ids.clone()))
        .exec(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "更新角色立绘文件路径失败",
            source,
        })?;
    if updated.rows_affected != portrait_ids.len() as u64 {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "更新角色立绘文件路径数量不一致: updated={}, expected={}",
            updated.rows_affected,
            portrait_ids.len()
        )));
    }
    Ok(())
}

async fn insert_generated_images<Importer>(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
    media_importer: &mut Importer,
) -> Result<(), ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    if package.generated_images.is_empty() {
        return Ok(());
    }
    let rows = package
        .generated_images
        .iter()
        .map(|image| {
            let mut active: generated_image::ActiveModel = image.clone().into();
            active.id = sea_orm::NotSet;
            active.project_id = Set(map_required(
                &maps.project_ids,
                image.project_id,
                "生成图片 project_id",
            )?);
            active.chapter_id = Set(map_optional(
                &maps.chapter_ids,
                image.chapter_id,
                "生成图片 chapter_id",
            )?);
            active.file_path = Set(String::new());
            Ok::<_, ProjectFamilyImportError>(active)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let inserted = generated_image::Entity::insert_many(rows)
        .exec_with_returning_many(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "导入生成图片失败",
            source,
        })?;
    if inserted.len() != package.generated_images.len() {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "导入生成图片批量写入返回行数不一致: inserted={}, expected={}",
            inserted.len(),
            package.generated_images.len()
        )));
    }
    let mut file_path_case = CaseStatement::new();
    let mut image_ids = Vec::with_capacity(inserted.len());
    for (image, inserted) in package.generated_images.iter().zip(inserted) {
        let file_path = media_importer
            .write_imported_media_field(
                inserted.project_id,
                CanvasNodeRefKind::GeneratedImage.as_str(),
                image.id,
                inserted.id,
                FILE_PATH_FIELD,
            )?
            .ok_or_else(|| {
                ProjectFamilyImportError::invalid_package(format!(
                    "生成图片缺少媒体文件: {}",
                    image.id
                ))
            })?;
        file_path_case = file_path_case.case(
            generated_image::Column::Id.eq(inserted.id),
            Expr::value(file_path.clone()),
        );
        image_ids.push(inserted.id);
        maps.generated_image_ids.insert(image.id, inserted.id);
        maps.generated_image_paths.insert(image.id, file_path);
    }
    let updated = generated_image::Entity::update_many()
        .col_expr(
            generated_image::Column::FilePath,
            file_path_case
                .finally(Expr::col(generated_image::Column::FilePath))
                .into(),
        )
        .filter(generated_image::Column::Id.is_in(image_ids.clone()))
        .exec(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "更新生成图片文件路径失败",
            source,
        })?;
    if updated.rows_affected != image_ids.len() as u64 {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "更新生成图片文件路径数量不一致: updated={}, expected={}",
            updated.rows_affected,
            image_ids.len()
        )));
    }
    Ok(())
}

async fn insert_manga_prompts(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_mapped_import_rows!(
        txn,
        &package.chapter_manga_prompts,
        prompt,
        chapter_manga_prompt::ActiveModel,
        maps.chapter_manga_prompt_ids,
        "导入章节漫画提示词失败",
        |active| {
            let layout = prompt.layout.as_ref().ok_or_else(|| {
                ProjectFamilyImportError::invalid_package(format!(
                    "章节漫画提示词 #{} 缺少 layout",
                    prompt.id
                ))
            })?;
            active.project_id = Set(map_required(
                &maps.project_ids,
                prompt.project_id,
                "章节漫画提示词 project_id",
            )?);
            active.chapter_id = Set(map_required(
                &maps.chapter_ids,
                prompt.chapter_id,
                "章节漫画提示词 chapter_id",
            )?);
            let page_number = u32::try_from(prompt.page_number).map_err(|_| {
                ProjectFamilyImportError::invalid_package(format!(
                    "章节漫画提示词 #{} 的 page_number 无效: {}",
                    prompt.id, prompt.page_number
                ))
            })?;
            if page_number == 0 {
                return Err(ProjectFamilyImportError::invalid_package(format!(
                    "章节漫画提示词 #{} 的 page_number 必须大于 0",
                    prompt.id
                )));
            }
            active.layout = Set(Some(remap_manga_page_layout(
                layout,
                maps,
                page_number,
                &format!("章节漫画提示词 #{} layout", prompt.id),
            )?));
            active.generated_image_id = Set(map_optional(
                &maps.generated_image_ids,
                prompt.generated_image_id,
                "章节漫画提示词 generated_image_id",
            )?);
        }
    );
    Ok(())
}

async fn insert_manga_pipeline_artifacts(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_import_rows!(
        txn,
        &package.manga_pipeline_artifacts,
        artifact,
        manga_pipeline_artifact::ActiveModel,
        "导入漫画阶段产物失败",
        |active| {
            active.project_id = Set(map_required(
                &maps.project_ids,
                artifact.project_id,
                "漫画阶段产物 project_id",
            )?);
            active.chapter_id = Set(map_required(
                &maps.chapter_ids,
                artifact.chapter_id,
                "漫画阶段产物 chapter_id",
            )?);
            let stage = a4n_domain::manga::MangaPipelineArtifactStage::from_key(&artifact.stage)
                .ok_or_else(|| {
                    ProjectFamilyImportError::invalid_package(format!(
                        "漫画阶段产物 #{} 的 stage 无效: {}",
                        artifact.id, artifact.stage
                    ))
                })?;
            active.payload = Set(remap_manga_pipeline_artifact_payload(
                stage,
                &artifact.payload,
                maps,
                &format!("漫画阶段产物 #{} payload", artifact.id),
            )?);
        }
    );
    insert_legacy_v3_manga_artifacts(txn, package, maps).await?;
    Ok(())
}

async fn insert_legacy_v3_manga_artifacts(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    for prompt in &package.chapter_manga_prompts {
        let Some(value) = prompt.layout.as_ref().filter(|layout| {
            layout
                .get("schema_version")
                .and_then(serde_json::Value::as_str)
                == Some("manga_page_generation_v3")
        }) else {
            continue;
        };
        let layout: MangaPageGenerationLayoutV3 =
            serde_json::from_value(value.clone()).map_err(|error| {
                ProjectFamilyImportError::invalid_package(format!(
                    "章节漫画提示词 #{} 的 v3 layout 无效: {error}",
                    prompt.id
                ))
            })?;
        let page_number = u32::try_from(prompt.page_number)
            .ok()
            .filter(|page_number| *page_number > 0)
            .ok_or_else(|| {
                ProjectFamilyImportError::invalid_package(format!(
                    "章节漫画提示词 #{} 的 page_number 无效",
                    prompt.id
                ))
            })?;
        layout.validate(page_number).map_err(|error| {
            ProjectFamilyImportError::invalid_package(format!(
                "章节漫画提示词 #{} 的 v3 layout 无效: {error}",
                prompt.id
            ))
        })?;
        let project_id = map_required(
            &maps.project_ids,
            prompt.project_id,
            "旧版漫画阶段产物 project_id",
        )?;
        let chapter_id = map_required(
            &maps.chapter_ids,
            prompt.chapter_id,
            "旧版漫画阶段产物 chapter_id",
        )?;
        let mut legacy_artifacts = vec![
            (
                MangaPipelineArtifactStage::Storyboard,
                serde_json::to_value(&layout.page_plan).map_err(|error| {
                    ProjectFamilyImportError::invalid_package(format!(
                        "章节漫画提示词 #{} 的 v3 分镜无法编码: {error}",
                        prompt.id
                    ))
                })?,
            ),
            (
                MangaPipelineArtifactStage::DrawingPrompt,
                serde_json::to_value(&layout.drawing_prompt).map_err(|error| {
                    ProjectFamilyImportError::invalid_package(format!(
                        "章节漫画提示词 #{} 的 v3 绘图提示词无法编码: {error}",
                        prompt.id
                    ))
                })?,
            ),
        ];
        if !layout.reference_bindings.is_empty() {
            legacy_artifacts.push((
                MangaPipelineArtifactStage::ReferenceBindings,
                serde_json::to_value(&layout.reference_bindings).map_err(|error| {
                    ProjectFamilyImportError::invalid_package(format!(
                        "章节漫画提示词 #{} 的 v3 参考绑定无法编码: {error}",
                        prompt.id
                    ))
                })?,
            ));
        }
        if let Some(image_id) = prompt.generated_image_id {
            let image = package
                .generated_images
                .iter()
                .find(|image| image.id == image_id)
                .ok_or_else(|| {
                    ProjectFamilyImportError::invalid_package(format!(
                        "章节漫画提示词 #{} 引用了未导入的成图 {}",
                        prompt.id, image_id
                    ))
                })?;
            if image.image_type != "manga_page"
                || image.project_id != prompt.project_id
                || image
                    .chapter_id
                    .is_some_and(|image_chapter_id| image_chapter_id != prompt.chapter_id)
            {
                return Err(ProjectFamilyImportError::invalid_package(format!(
                    "章节漫画提示词 #{} 引用的图片 {} 不属于当前漫画页",
                    prompt.id, image_id
                )));
            }
            legacy_artifacts.push((
                MangaPipelineArtifactStage::Image,
                serde_json::to_value(MangaPageImageArtifactV1 {
                    workflow_run_id: None,
                    generated_image_id: image_id,
                    width: image.width,
                    height: image.height,
                })
                .map_err(|error| {
                    ProjectFamilyImportError::invalid_package(format!(
                        "章节漫画提示词 #{} 的成图产物无法编码: {error}",
                        prompt.id
                    ))
                })?,
            ));
        }

        for (stage, payload) in legacy_artifacts {
            let payload = remap_manga_pipeline_artifact_payload(
                stage,
                &payload,
                maps,
                &format!("章节漫画提示词 #{} 的 v3 {}", prompt.id, stage.key()),
            )?;
            let existing = manga_pipeline_artifact::Entity::find()
                .filter(manga_pipeline_artifact::Column::ProjectId.eq(project_id))
                .filter(manga_pipeline_artifact::Column::ChapterId.eq(chapter_id))
                .filter(manga_pipeline_artifact::Column::Stage.eq(stage.key()))
                .filter(manga_pipeline_artifact::Column::PageNumber.eq(prompt.page_number))
                .one(txn)
                .await
                .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                    context: "读取已导入的漫画阶段产物失败",
                    source,
                })?;
            if let Some(existing) = existing {
                if existing.payload != payload {
                    return Err(ProjectFamilyImportError::invalid_package(format!(
                        "章节漫画提示词 #{} 的 v3 {} 与阶段产物记录不一致",
                        prompt.id,
                        stage.key()
                    )));
                }
                continue;
            }
            let payload_json = serde_json::to_string(&payload).map_err(|error| {
                ProjectFamilyImportError::invalid_package(format!(
                    "章节漫画提示词 #{} 的 v3 {} 无法编码: {error}",
                    prompt.id,
                    stage.key()
                ))
            })?;
            manga_pipeline_artifact::ActiveModel {
                project_id: Set(project_id),
                chapter_id: Set(chapter_id),
                stage: Set(stage.key().into()),
                page_number: Set(prompt.page_number),
                input_fingerprint: Set(source_text_hash(&format!(
                    "legacy:project_import:manga_page:{}:{}:{}",
                    prompt.id,
                    stage.key(),
                    page_number
                ))),
                output_fingerprint: Set(source_text_hash(&payload_json)),
                payload: Set(payload),
                stale: Set(true),
                created_at: Set(prompt.created_at),
                updated_at: Set(prompt.updated_at),
                ..Default::default()
            }
            .insert(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "导入旧版漫画阶段产物失败",
                source,
            })?;
        }
    }
    Ok(())
}

async fn update_generated_image_metadata(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let mut metadata_case = CaseStatement::new();
    let mut image_ids = Vec::new();
    for image in &package.generated_images {
        if image.image_type != "manga_page" {
            continue;
        }
        let Some(metadata) = image.metadata_json.as_ref() else {
            continue;
        };
        let new_image_id = map_required(&maps.generated_image_ids, image.id, "生成图片 ID")?;
        metadata_case = metadata_case.case(
            generated_image::Column::Id.eq(new_image_id),
            Expr::value(Some(remap_manga_generated_image_metadata(metadata, maps)?))
                .cast_as(Alias::new("json")),
        );
        image_ids.push(new_image_id);
    }
    if image_ids.is_empty() {
        return Ok(());
    }
    let updated = generated_image::Entity::update_many()
        .col_expr(
            generated_image::Column::MetadataJson,
            metadata_case
                .finally(Expr::col(generated_image::Column::MetadataJson))
                .into(),
        )
        .filter(generated_image::Column::Id.is_in(image_ids.clone()))
        .exec(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "更新生成图片 metadata_json 失败",
            source,
        })?;
    if updated.rows_affected != image_ids.len() as u64 {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "更新生成图片 metadata_json 数量不一致: updated={}, expected={}",
            updated.rows_affected,
            image_ids.len()
        )));
    }
    Ok(())
}
