use super::*;

mod generation_inputs;

pub(super) use self::generation_inputs::enrich_lineage_records_with_generation_inputs;

pub(super) async fn blueprint_detail_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let blueprints = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .order_by_asc(novel_blueprint::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(blueprints
        .into_iter()
        .map(|blueprint| AssetLineageRecord {
            id: format!("novel_blueprint:{}", blueprint.id),
            label: blueprint.title.clone(),
            asset_kind: "novel_blueprint".to_string(),
            asset_id: Some(blueprint.id),
            status: Some(blueprint.current_version.clone()),
            origin: "llm_generated".to_string(),
            source_refs: vec![lineage_ref(
                "inspiration",
                "灵感对话",
                "text",
                "inspiration_conversation",
                None,
            )],
            prompt_sources: optional_prompt_sources(vec![prompt_source(
                &format!("novel_blueprint_synopsis:{}", blueprint.id),
                "blueprint_synopsis",
                blueprint.full_synopsis.as_deref(),
                Some(blueprint.id),
                None,
            )]),
            media_sources: Vec::new(),
            request_snapshot: None,
            input_status: "missing".to_string(),
            generation_runs: Vec::new(),
            generation_inputs: Vec::new(),
            result: db_record_result("novel_blueprint", &blueprint),
            warnings: Vec::new(),
        })
        .collect())
}

pub(super) async fn blueprint_version_detail_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let versions = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_version::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(versions
        .into_iter()
        .map(|version| AssetLineageRecord {
            id: format!("blueprint_version:{}", version.id),
            label: format!("蓝图版本 {}", version.version),
            asset_kind: "blueprint_version".to_string(),
            asset_id: Some(version.id),
            status: Some(version.version.clone()),
            origin: "derived_text".to_string(),
            source_refs: vec![lineage_ref(
                "blueprint",
                "故事蓝图",
                "text",
                "novel_blueprint",
                None,
            )],
            prompt_sources: Vec::new(),
            media_sources: Vec::new(),
            request_snapshot: None,
            input_status: "missing".to_string(),
            generation_runs: Vec::new(),
            generation_inputs: Vec::new(),
            result: db_record_result("blueprint_version", &version),
            warnings: Vec::new(),
        })
        .collect())
}

pub(super) async fn part_outline_detail_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(parts
        .into_iter()
        .map(|part| AssetLineageRecord {
            id: format!("part_outline:{}", part.id),
            label: format!("第 {} 部：{}", part.part_number, part.title),
            asset_kind: "part_outline".to_string(),
            asset_id: Some(part.id),
            status: None,
            origin: "llm_generated".to_string(),
            source_refs: vec![lineage_ref(
                "blueprint",
                "故事蓝图",
                "text",
                "novel_blueprint",
                None,
            )],
            prompt_sources: optional_prompt_sources(vec![prompt_source(
                &format!("part_outline_summary:{}", part.id),
                "part_outline_summary",
                part.summary.as_deref(),
                Some(part.id),
                None,
            )]),
            media_sources: Vec::new(),
            request_snapshot: None,
            input_status: "missing".to_string(),
            generation_runs: Vec::new(),
            generation_inputs: Vec::new(),
            result: db_record_result("part_outline", &part),
            warnings: Vec::new(),
        })
        .collect())
}

pub(super) async fn chapter_outline_detail_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(outlines
        .into_iter()
        .map(|outline| {
            let source_refs = if let Some(part_id) = outline.part_id {
                vec![lineage_ref(
                    &format!("part_outline:{part_id}"),
                    "分部大纲",
                    "text",
                    "part_outline",
                    Some(part_id),
                )]
            } else {
                vec![lineage_ref(
                    "blueprint",
                    "故事蓝图",
                    "text",
                    "novel_blueprint",
                    None,
                )]
            };
            AssetLineageRecord {
                id: format!("chapter_outline:{}", outline.id),
                label: outline
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("第 {} 章大纲", outline.chapter_number)),
                asset_kind: "chapter_outline".to_string(),
                asset_id: Some(outline.id),
                status: Some(outline.status.clone()),
                origin: "llm_generated".to_string(),
                source_refs,
                prompt_sources: optional_prompt_sources(vec![prompt_source(
                    &format!("chapter_outline_summary:{}", outline.id),
                    "chapter_outline_summary",
                    outline.summary.as_deref(),
                    Some(outline.id),
                    None,
                )]),
                media_sources: Vec::new(),
                request_snapshot: None,
                input_status: if outline
                    .summary
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    "complete"
                } else {
                    "missing"
                }
                .to_string(),
                generation_runs: Vec::new(),
                generation_inputs: Vec::new(),
                result: db_record_result("chapter_outline", &outline),
                warnings: Vec::new(),
            }
        })
        .collect())
}

pub(super) async fn chapter_content_detail_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(chapters
        .into_iter()
        .map(|chapter| AssetLineageRecord {
            id: format!("chapter:{}", chapter.id),
            label: chapter
                .title
                .clone()
                .unwrap_or_else(|| format!("第 {} 章正文", chapter.chapter_number)),
            asset_kind: "chapter_content".to_string(),
            asset_id: Some(chapter.id),
            status: Some(chapter.status.clone()),
            origin: if chapter.status == chapter::STATUS_IMPORTED {
                "imported"
            } else {
                "llm_generated"
            }
            .to_string(),
            source_refs: vec![lineage_ref(
                "chapter_outlines",
                "章节大纲",
                "text",
                "chapter_outline",
                chapter.outline_id,
            )],
            prompt_sources: optional_prompt_sources(vec![prompt_source(
                &format!("chapter_content:{}", chapter.id),
                "chapter_content",
                Some(&chapter.content),
                Some(chapter.id),
                None,
            )]),
            media_sources: Vec::new(),
            request_snapshot: None,
            input_status: "missing".to_string(),
            generation_runs: Vec::new(),
            generation_inputs: Vec::new(),
            result: db_record_result("chapter", &chapter),
            warnings: Vec::new(),
        })
        .collect())
}

pub(super) async fn chapter_analysis_detail_records<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(chapters
        .into_iter()
        .filter(|chapter| {
            chapter
                .analysis_data
                .as_ref()
                .is_some_and(|value| !value.is_null())
        })
        .map(|chapter| AssetLineageRecord {
            id: format!("chapter_analysis:{}", chapter.id),
            label: format!("第 {} 章分析", chapter.chapter_number),
            asset_kind: "chapter_analysis".to_string(),
            asset_id: Some(chapter.id),
            status: None,
            origin: "llm_generated".to_string(),
            source_refs: vec![lineage_ref(
                &format!("chapter:{}", chapter.id),
                "章节正文",
                "text",
                "chapter_content",
                Some(chapter.id),
            )],
            prompt_sources: Vec::new(),
            media_sources: Vec::new(),
            request_snapshot: None,
            input_status: "missing".to_string(),
            generation_runs: Vec::new(),
            generation_inputs: Vec::new(),
            result: db_record_result("chapter", &chapter),
            warnings: Vec::new(),
        })
        .collect())
}

pub(super) async fn character_profile_detail_records<C>(
    db: &C,
    project_id: i64,
    record_id: Option<i64>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let Some(profile_id) = record_id else {
        return Ok(Vec::new());
    };
    let profile = protagonist_profile::Entity::find_by_id(profile_id)
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("人物档案不存在: {profile_id}")))?;
    let text_discovered = profile.is_text_discovered() || profile.last_synced_chapter > 0;
    let mut source_refs = if text_discovered {
        vec![lineage_ref(
            "chapter_analysis",
            "章节分析",
            "text",
            "chapter_analysis",
            None,
        )]
    } else {
        vec![lineage_ref(
            "blueprint",
            "故事蓝图",
            "text",
            "novel_blueprint",
            None,
        )]
    };
    source_refs.push(lineage_ref(
        "character_profiles",
        "人物档案",
        "text",
        "character_profile",
        None,
    ));
    Ok(vec![AssetLineageRecord {
        id: format!("character_profile:{}", profile.id),
        label: profile.name.clone(),
        asset_kind: "character_profile_record".to_string(),
        asset_id: Some(profile.id),
        status: None,
        origin: if text_discovered {
            "llm_generated"
        } else {
            "derived_text"
        }
        .to_string(),
        source_refs,
        prompt_sources: optional_prompt_sources(vec![
            prompt_source(
                "appearance",
                "manual_description",
                profile.appearance.as_deref(),
                Some(profile.id),
                None,
            ),
            prompt_source(
                "personality",
                "manual_description",
                profile.personality.as_deref(),
                Some(profile.id),
                None,
            ),
            prompt_source(
                "background",
                "manual_description",
                profile.background.as_deref(),
                Some(profile.id),
                None,
            ),
        ]),
        media_sources: Vec::new(),
        request_snapshot: None,
        input_status: "missing".to_string(),
        generation_runs: Vec::new(),
        generation_inputs: Vec::new(),
        result: db_record_result("protagonist_profile", &profile),
        warnings: Vec::new(),
    }])
}

pub(super) async fn object_profile_detail_records<C>(
    db: &C,
    project_id: i64,
    record_id: Option<i64>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let Some(profile_id) = record_id else {
        return Ok(Vec::new());
    };
    let profile = object_profile::Entity::find_by_id(profile_id)
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("物体档案不存在: {profile_id}")))?;
    Ok(vec![AssetLineageRecord {
        id: format!("object_profile:{}", profile.id),
        label: profile.name.clone(),
        asset_kind: "object_profile_record".to_string(),
        asset_id: Some(profile.id),
        status: None,
        origin: "llm_generated".to_string(),
        source_refs: vec![
            lineage_ref(
                "chapter_analysis",
                "章节分析",
                "text",
                "chapter_analysis",
                None,
            ),
            lineage_ref(
                "object_profiles",
                "物体档案",
                "text",
                "object_profile",
                None,
            ),
        ],
        prompt_sources: optional_prompt_sources(vec![
            prompt_source(
                "visual_description",
                "manual_description",
                Some(&profile.visual_description),
                Some(profile.id),
                None,
            ),
            prompt_source(
                "image_prompt_seed",
                "manual_description",
                Some(&profile.image_prompt_seed),
                Some(profile.id),
                None,
            ),
        ]),
        media_sources: Vec::new(),
        request_snapshot: None,
        input_status: "missing".to_string(),
        generation_runs: Vec::new(),
        generation_inputs: Vec::new(),
        result: db_record_result("object_profile", &profile),
        warnings: Vec::new(),
    }])
}

pub(super) async fn scene_profile_detail_records<C>(
    db: &C,
    project_id: i64,
    record_id: Option<i64>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let Some(profile_id) = record_id else {
        return Ok(Vec::new());
    };
    let profile = scene_profile::Entity::find_by_id(profile_id)
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("场景档案不存在: {profile_id}")))?;
    Ok(vec![AssetLineageRecord {
        id: format!("scene_profile:{}", profile.id),
        label: profile.name.clone(),
        asset_kind: "scene_profile_record".to_string(),
        asset_id: Some(profile.id),
        status: None,
        origin: "llm_generated".to_string(),
        source_refs: vec![
            lineage_ref(
                "chapter_analysis",
                "章节分析",
                "text",
                "chapter_analysis",
                None,
            ),
            lineage_ref("scene_profiles", "场景档案", "text", "scene_profile", None),
        ],
        prompt_sources: optional_prompt_sources(vec![
            prompt_source(
                "visual_description",
                "manual_description",
                Some(&profile.visual_description),
                Some(profile.id),
                None,
            ),
            prompt_source(
                "image_prompt_seed",
                "manual_description",
                Some(&profile.image_prompt_seed),
                Some(profile.id),
                None,
            ),
        ]),
        media_sources: Vec::new(),
        request_snapshot: None,
        input_status: "missing".to_string(),
        generation_runs: Vec::new(),
        generation_inputs: Vec::new(),
        result: db_record_result("scene_profile", &profile),
        warnings: Vec::new(),
    }])
}

pub(super) async fn character_portrait_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    profile_id: Option<i64>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let Some(profile_id) = profile_id else {
        return Ok(Vec::new());
    };
    let portraits = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .order_by_asc(character_portrait::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_image::Column::ImageType.eq("portrait"))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(portraits
        .into_iter()
        .map(|portrait| {
            let mirror = images.iter().find(|image| {
                image
                    .metadata_json
                    .as_ref()
                    .and_then(|value| value.get("portrait_id"))
                    .and_then(Value::as_i64)
                    == Some(portrait.id)
            });
            let variant_kind = mirror
                .and_then(|image| image.metadata_json.as_ref())
                .and_then(|value| value.get("variant_kind"))
                .and_then(Value::as_str);
            AssetLineageRecord {
                id: format!("character_portrait:{}", portrait.id),
                label: format!("{} #{}", portrait.portrait_type, portrait.id),
                asset_kind: "character_portrait".to_string(),
                asset_id: Some(portrait.id),
                status: None,
                origin: if variant_kind == Some("manual_upload") {
                    "uploaded".to_string()
                } else {
                    "image_generated".to_string()
                },
                source_refs: vec![lineage_ref(
                    &format!("character_profile:{profile_id}"),
                    "人物档案",
                    "text",
                    "character_profile_record",
                    Some(profile_id),
                )],
                prompt_sources: optional_prompt_sources(vec![prompt_source(
                    &format!("character_portrait_prompt:{}", portrait.id),
                    if variant_kind == Some("manual_upload") {
                        "manual_description"
                    } else {
                        "character_portrait_prompt"
                    },
                    Some(&portrait.prompt),
                    Some(portrait.id),
                    None,
                )]),
                media_sources: mirror
                    .map(|image| {
                        vec![lineage_ref(
                            &format!("generated_image:{}", image.id),
                            "镜像图片记录",
                            "image",
                            "generated_image",
                            Some(image.id),
                        )]
                    })
                    .unwrap_or_default(),
                request_snapshot: mirror
                    .and_then(|image| image.metadata_json.as_ref())
                    .and_then(|value| value.get("generation").cloned()),
                input_status: if mirror.is_some() {
                    "complete"
                } else {
                    "missing"
                }
                .to_string(),
                generation_runs: Vec::new(),
                generation_inputs: Vec::new(),
                result: db_record_result_with_related(
                    "character_portrait",
                    &portrait,
                    mirror
                        .map(|image| db_related_record("generated_image", image))
                        .into_iter()
                        .collect(),
                ),
                warnings: if mirror.is_none() {
                    vec!["未找到对应 generated_image 镜像记录。".to_string()]
                } else {
                    Vec::new()
                },
            }
        })
        .collect())
}

pub(super) async fn profile_material_image_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    profile_id: Option<i64>,
    profile_kind: &str,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let Some(profile_id) = profile_id else {
        return Ok(Vec::new());
    };
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(images
        .into_iter()
        .filter(|image| {
            crate::profile_material::generated_image_profile_material_identity(image)
                == Some((profile_kind, profile_id))
        })
        .map(|image| generated_image_record(image, "profile_material_image", "image_generated"))
        .collect())
}

pub(super) async fn manga_prompt_detail_records<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(chapter_manga_prompt::Column::PromptType.ne("pipeline_status"))
        .order_by_asc(chapter_manga_prompt::Column::ChapterId)
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(prompts
        .into_iter()
        .map(|prompt| manga_prompt_record(prompt, None))
        .collect())
}

pub(super) async fn manga_image_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    warnings: &mut Vec<String>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(chapter_manga_prompt::Column::PromptType.ne("pipeline_status"))
        .order_by_asc(chapter_manga_prompt::Column::ChapterId)
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
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

pub(super) async fn video_prompt_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    prompt_types: &[&str],
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(chapter_video_prompt::Column::PromptType.is_in(prompt_types.iter().copied()))
        .order_by_asc(chapter_video_prompt::Column::ChapterId)
        .order_by_asc(chapter_video_prompt::Column::WorkflowMode)
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(prompts.into_iter().map(video_prompt_record).collect())
}

pub(super) async fn video_frame_image_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    warnings: &mut Vec<String>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let prompt_by_id = prompts
        .iter()
        .map(|prompt| (prompt.id, prompt))
        .collect::<BTreeMap<_, _>>();
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut records = Vec::new();
    for image in images {
        if let Some(prompt_id) = video_frame_image_prompt_id(image.metadata_json.as_ref()) {
            let mut record =
                generated_image_record(image.clone(), "video_frame_image", "image_generated");
            if let Some(prompt) = prompt_by_id.get(&prompt_id) {
                record.source_refs.push(lineage_ref(
                    &format!("chapter_video_prompt:{}", prompt.id),
                    "视频首尾帧提示词",
                    "prompt",
                    "video_frame_prompt",
                    Some(prompt.id),
                ));
                record.prompt_sources = video_prompt_sources(prompt);
            } else {
                record.warnings.push(format!(
                    "图片 metadata 指向的视频提示词 #{} 不存在。",
                    prompt_id
                ));
            }
            records.push(record);
        } else if crate::profile_material::is_video_frame_input_metadata(
            image.metadata_json.as_ref(),
        ) {
            warnings.push(format!(
                "视频帧图片 #{} 缺少 generation_context.prompt_id。",
                image.id
            ));
            let mut record = generated_image_record(image, "video_frame_image", "unknown");
            record
                .warnings
                .push("未找到视频首尾帧提示词来源。".to_string());
            records.push(record);
        }
    }
    Ok(records)
}

pub(super) async fn generated_video_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    warnings: &mut Vec<String>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let prompt_by_video = prompts
        .iter()
        .filter_map(|prompt| prompt.generated_video_id.map(|id| (id, prompt)))
        .collect::<BTreeMap<_, _>>();
    let prompt_by_id = prompts
        .iter()
        .map(|prompt| (prompt.id, prompt))
        .collect::<BTreeMap<_, _>>();
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_asc(generated_video::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(videos
        .into_iter()
        .map(|video| {
            let prompt = prompt_by_video.get(&video.id).copied().or_else(|| {
                generated_video_prompt_id(video.metadata_json.as_ref())
                    .and_then(|id| prompt_by_id.get(&id).copied())
            });
            if prompt.is_none() {
                warnings.push(format!(
                    "生成视频 #{} 未找到 chapter_video_prompt 来源。",
                    video.id
                ));
            }
            let mut prompt_sources = Vec::new();
            if let Some(prompt) = prompt {
                prompt_sources = video_prompt_sources(prompt);
            }
            prompt_sources.extend(optional_prompt_sources(vec![
                prompt_source(
                    &format!("generated_video_prompt:{}", video.id),
                    "stored_generated_video_prompt",
                    Some(&video.prompt),
                    Some(video.id),
                    None,
                ),
                prompt_source(
                    &format!("generated_video_negative_prompt:{}", video.id),
                    "video_negative_prompt",
                    video.negative_prompt.as_deref(),
                    Some(video.id),
                    None,
                ),
            ]));
            let mut media_sources = Vec::new();
            if let Some(id) = video.source_image_id {
                media_sources.push(lineage_ref(
                    &format!("generated_image:{id}"),
                    "源图片",
                    "image",
                    "generated_image",
                    Some(id),
                ));
            }
            if let Some(id) = video.source_video_id {
                media_sources.push(lineage_ref(
                    &format!("generated_video:{id}"),
                    "源视频",
                    "video",
                    "generated_video",
                    Some(id),
                ));
            }
            let request_snapshot = video
                .metadata_json
                .as_ref()
                .and_then(|value| value.get("provider_request_snapshot").cloned());
            let input_complete =
                prompt.is_some() || !media_sources.is_empty() || request_snapshot.is_some();
            AssetLineageRecord {
                id: format!("generated_video:{}", video.id),
                label: format!("生成视频 #{}", video.id),
                asset_kind: "generated_video".to_string(),
                asset_id: Some(video.id),
                status: Some(video.status.clone()),
                origin: "video_generated".to_string(),
                source_refs: prompt
                    .map(|prompt| {
                        vec![lineage_ref(
                            &format!("chapter_video_prompt:{}", prompt.id),
                            "视频生成提示词",
                            "prompt",
                            "chapter_video_prompt",
                            Some(prompt.id),
                        )]
                    })
                    .unwrap_or_default(),
                prompt_sources,
                media_sources,
                request_snapshot,
                input_status: if input_complete {
                    "complete"
                } else {
                    "missing"
                }
                .to_string(),
                generation_runs: Vec::new(),
                generation_inputs: Vec::new(),
                result: db_record_result_with_related(
                    "generated_video",
                    &video,
                    prompt
                        .map(|prompt| db_related_record("chapter_video_prompt", prompt))
                        .into_iter()
                        .collect(),
                ),
                warnings: if prompt.is_none() {
                    vec!["未找到视频提示词来源，只能展示视频记录内保存的提示词。".to_string()]
                } else {
                    Vec::new()
                },
            }
        })
        .collect())
}

pub(super) async fn prompt_artifact_detail_records<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_asc(generation_spec::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    let spec_by_id = specs
        .iter()
        .map(|spec| (spec.id, spec))
        .collect::<BTreeMap<_, _>>();
    let artifacts = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(specs.iter().map(|spec| spec.id)))
        .order_by_asc(prompt_artifact::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(artifacts
        .into_iter()
        .map(|artifact| {
            let spec = spec_by_id.get(&artifact.generation_spec_id).copied();
            AssetLineageRecord {
                id: format!("prompt_artifact:{}", artifact.id),
                label: format!("{} #{}", artifact.prompt_kind, artifact.id),
                asset_kind: "prompt_artifact".to_string(),
                asset_id: Some(artifact.id),
                status: None,
                origin: "llm_generated".to_string(),
                source_refs: spec
                    .map(|spec| {
                        vec![lineage_ref(
                            &format!("generation_spec:{}", spec.id),
                            "统一生成规格",
                            "text",
                            "generation_spec",
                            Some(spec.id),
                        )]
                    })
                    .unwrap_or_default(),
                prompt_sources: optional_prompt_sources(vec![prompt_source(
                    &format!("prompt_artifact_text:{}", artifact.id),
                    "prompt_artifact",
                    Some(&artifact.prompt_text),
                    Some(artifact.id),
                    Some(artifact.edited_by_user),
                )]),
                media_sources: Vec::new(),
                request_snapshot: None,
                input_status: if spec.is_some() {
                    "complete"
                } else {
                    "missing"
                }
                .to_string(),
                generation_runs: Vec::new(),
                generation_inputs: Vec::new(),
                result: db_record_result_with_related(
                    "prompt_artifact",
                    &artifact,
                    spec.map(|spec| db_related_record("generation_spec", spec))
                        .into_iter()
                        .collect(),
                ),
                warnings: if spec.is_none() {
                    vec!["提示词产物指向的 generation_spec 不存在。".to_string()]
                } else {
                    Vec::new()
                },
            }
        })
        .collect())
}

pub(super) async fn generation_spec_detail_records<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_asc(generation_spec::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(specs
        .into_iter()
        .map(|spec| {
            let legacy_label = spec
                .params_json
                .get("legacy_table")
                .and_then(Value::as_str)
                .zip(spec.params_json.get("legacy_id").and_then(Value::as_i64))
                .map(|(table, id)| format!("{table} #{id}"));
            AssetLineageRecord {
                id: format!("generation_spec:{}", spec.id),
                label: legacy_label.unwrap_or_else(|| format!("{} #{}", spec.target_kind, spec.id)),
                asset_kind: "generation_spec".to_string(),
                asset_id: Some(spec.id),
                status: Some(spec.status.clone()),
                origin: "llm_generated".to_string(),
                source_refs: spec
                    .source_chapter_id
                    .map(|chapter_id| {
                        vec![lineage_ref(
                            &format!("chapter:{chapter_id}"),
                            "源章节",
                            "text",
                            "chapter",
                            Some(chapter_id),
                        )]
                    })
                    .unwrap_or_default(),
                prompt_sources: Vec::new(),
                media_sources: Vec::new(),
                request_snapshot: None,
                input_status: if spec.source_chapter_id.is_none()
                    || spec.source_snapshot_chapter_id.is_some()
                {
                    "complete"
                } else {
                    "missing"
                }
                .to_string(),
                generation_runs: Vec::new(),
                generation_inputs: Vec::new(),
                result: db_record_result("generation_spec", &spec),
                warnings: if spec.source_chapter_id.is_some()
                    && spec.source_snapshot_chapter_id.is_none()
                {
                    vec![
                        "生成规格缺少 source_snapshot_chapter_id，源快照章节链路不完整。"
                            .to_string(),
                    ]
                } else {
                    Vec::new()
                },
            }
        })
        .collect())
}

pub(super) async fn media_asset_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    warnings: &mut Vec<String>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_asc(media_asset::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if assets.is_empty() {
        return Ok(Vec::new());
    }
    let provenances = media_provenance::Entity::find()
        .filter(media_provenance::Column::MediaAssetId.is_in(assets.iter().map(|asset| asset.id)))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let provenance_by_asset = provenances
        .iter()
        .map(|provenance| (provenance.media_asset_id, provenance))
        .collect::<BTreeMap<_, _>>();
    let artifact_ids = provenances
        .iter()
        .filter_map(|row| row.prompt_artifact_id)
        .collect::<BTreeSet<_>>();
    let artifacts = if artifact_ids.is_empty() {
        Vec::new()
    } else {
        prompt_artifact::Entity::find()
            .filter(prompt_artifact::Column::Id.is_in(artifact_ids.iter().copied()))
            .all(db)
            .await
            .map_err(AppError::Database)?
    };
    let artifact_by_id = artifacts
        .iter()
        .map(|artifact| (artifact.id, artifact))
        .collect::<BTreeMap<_, _>>();

    Ok(assets
        .into_iter()
        .map(|asset| {
            let provenance = provenance_by_asset.get(&asset.id).copied();
            if provenance.is_none() {
                warnings.push(format!("媒体资产 #{} 缺少 media_provenance。", asset.id));
            }
            let artifact = provenance
                .and_then(|row| row.prompt_artifact_id)
                .and_then(|id| artifact_by_id.get(&id).copied());
            let origin = match provenance {
                Some(row) if row.imported_from_user => "imported",
                Some(row) if row.source_media_asset_id.is_some() => "extracted_frame",
                Some(_) if artifact.is_some() && asset.media_kind == "video" => "video_generated",
                Some(_) if artifact.is_some() => "image_generated",
                Some(_) if asset.origin == "generated" && asset.media_kind == "video" => {
                    "video_generated"
                }
                Some(_) if asset.origin == "generated" => "image_generated",
                Some(_) => "manual",
                None => "unknown",
            }
            .to_string();
            AssetLineageRecord {
                id: format!("media_asset:{}", asset.id),
                label: format!("{} #{}", asset.media_kind, asset.id),
                asset_kind: "media_asset".to_string(),
                asset_id: Some(asset.id),
                status: Some(asset.status.clone()),
                origin,
                source_refs: provenance
                    .and_then(|row| row.generation_spec_id)
                    .map(|id| {
                        vec![lineage_ref(
                            &format!("generation_spec:{id}"),
                            "统一生成规格",
                            "text",
                            "generation_spec",
                            Some(id),
                        )]
                    })
                    .unwrap_or_default(),
                prompt_sources: artifact
                    .map(|artifact| {
                        optional_prompt_sources(vec![prompt_source(
                            &format!("prompt_artifact_text:{}", artifact.id),
                            "prompt_artifact",
                            Some(&artifact.prompt_text),
                            Some(artifact.id),
                            Some(artifact.edited_by_user),
                        )])
                    })
                    .unwrap_or_default(),
                media_sources: provenance
                    .and_then(|row| row.source_media_asset_id)
                    .map(|id| {
                        vec![lineage_ref(
                            &format!("media_asset:{id}"),
                            "源媒体资产",
                            "media",
                            "media_asset",
                            Some(id),
                        )]
                    })
                    .unwrap_or_default(),
                request_snapshot: provenance.and_then(|row| row.request_snapshot_json.clone()),
                input_status: if provenance.is_some() {
                    "complete"
                } else {
                    "missing"
                }
                .to_string(),
                generation_runs: Vec::new(),
                generation_inputs: Vec::new(),
                result: db_record_result_with_related(
                    "media_asset",
                    &asset,
                    provenance
                        .map(|row| db_related_record("media_provenance", row))
                        .into_iter()
                        .chain(artifact.map(|row| db_related_record("prompt_artifact", row)))
                        .collect(),
                ),
                warnings: if provenance.is_none() {
                    vec!["未找到媒体来源记录。".to_string()]
                } else if provenance.and_then(|row| row.prompt_artifact_id).is_some()
                    && artifact.is_none()
                {
                    vec!["媒体来源指向的 prompt_artifact 不存在。".to_string()]
                } else {
                    Vec::new()
                },
            }
        })
        .collect())
}

pub(super) fn structural_source_refs_for_node(
    graph: &AssetDependencyGraph,
    node_id: &str,
) -> Vec<AssetLineageRef> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.to == node_id && edge.count > 0)
        .filter_map(|edge| {
            graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.from)
                .map(|source| {
                    lineage_ref(
                        &source.id,
                        &source.label,
                        &source.kind,
                        &source.asset_type,
                        source.record_id,
                    )
                })
        })
        .collect()
}

pub(super) fn generic_node_detail_records(
    node: &AssetGraphNode,
    source_refs: Vec<AssetLineageRef>,
) -> Vec<AssetLineageRecord> {
    vec![AssetLineageRecord {
        id: node.id.clone(),
        label: node.label.clone(),
        asset_kind: node.asset_type.clone(),
        asset_id: node.record_id,
        status: node.status.clone(),
        origin: if node.kind == "text" {
            "derived_text"
        } else {
            "unknown"
        }
        .to_string(),
        source_refs,
        prompt_sources: Vec::new(),
        media_sources: Vec::new(),
        request_snapshot: None,
        input_status: "missing".to_string(),
        generation_runs: Vec::new(),
        generation_inputs: Vec::new(),
        result: node.details.clone(),
        warnings: Vec::new(),
    }]
}

pub(super) fn lineage_ref(
    id: &str,
    label: &str,
    kind: &str,
    asset_type: &str,
    record_id: Option<i64>,
) -> AssetLineageRef {
    AssetLineageRef {
        id: id.to_string(),
        label: label.to_string(),
        kind: kind.to_string(),
        asset_type: asset_type.to_string(),
        record_id,
    }
}

pub(super) fn prompt_source(
    id: &str,
    kind: &str,
    text: Option<&str>,
    source_record_id: Option<i64>,
    edited_by_user: Option<bool>,
) -> Option<PromptSource> {
    let text = text?.trim();
    if text.is_empty() {
        return None;
    }
    Some(PromptSource {
        id: id.to_string(),
        kind: kind.to_string(),
        text: text.to_string(),
        source_record_id,
        edited_by_user,
    })
}

pub(super) fn optional_prompt_sources(items: Vec<Option<PromptSource>>) -> Vec<PromptSource> {
    items.into_iter().flatten().collect()
}

pub(super) fn db_record_result<T>(table: &str, record: &T) -> Value
where
    T: Serialize,
{
    json!({
        "table": table,
        "record": db_record_value(record),
    })
}

pub(super) fn db_record_result_with_related<T>(
    table: &str,
    record: &T,
    related_records: Vec<Value>,
) -> Value
where
    T: Serialize,
{
    json!({
        "table": table,
        "record": db_record_value(record),
        "related_records": related_records,
    })
}

pub(super) fn db_related_record<T>(table: &str, record: &T) -> Value
where
    T: Serialize,
{
    json!({
        "table": table,
        "record": db_record_value(record),
    })
}

pub(super) fn db_record_value<T>(record: &T) -> Value
where
    T: Serialize,
{
    serde_json::to_value(record)
        .unwrap_or_else(|err| json!({ "serialization_error": err.to_string() }))
}

pub(super) fn generated_image_result(image: &generated_image::Model) -> Value {
    db_record_result("generated_image", image)
}

pub(super) fn generated_image_record(
    image: generated_image::Model,
    asset_kind: &str,
    origin: &str,
) -> AssetLineageRecord {
    AssetLineageRecord {
        id: format!("generated_image:{}", image.id),
        label: format!("{} #{}", image.image_type, image.id),
        asset_kind: asset_kind.to_string(),
        asset_id: Some(image.id),
        status: None,
        origin: origin.to_string(),
        source_refs: image
            .chapter_id
            .map(|id| {
                vec![lineage_ref(
                    &format!("chapter:{id}"),
                    "章节正文",
                    "text",
                    "chapter_content",
                    Some(id),
                )]
            })
            .unwrap_or_default(),
        prompt_sources: optional_prompt_sources(vec![prompt_source(
            &format!("generated_image_prompt:{}", image.id),
            "image_generation_prompt",
            Some(&image.prompt),
            Some(image.id),
            None,
        )]),
        media_sources: generated_image_media_sources(&image),
        request_snapshot: image
            .metadata_json
            .as_ref()
            .and_then(|value| {
                value
                    .get("provider_request_snapshot")
                    .or_else(|| value.get("request_snapshot"))
                    .or_else(|| value.get("generation"))
            })
            .cloned(),
        input_status: "missing".to_string(),
        generation_runs: Vec::new(),
        generation_inputs: Vec::new(),
        result: generated_image_result(&image),
        warnings: Vec::new(),
    }
}

pub(super) fn generated_image_media_sources(
    image: &generated_image::Model,
) -> Vec<AssetLineageRef> {
    let Some(metadata) = image.metadata_json.as_ref() else {
        return Vec::new();
    };
    let mut refs = Vec::new();
    if let Some(ids) = metadata
        .get("reference_image_ids")
        .and_then(Value::as_array)
    {
        refs.extend(ids.iter().filter_map(Value::as_i64).map(|id| {
            lineage_ref(
                &format!("generated_image:{id}"),
                "参考图片",
                "image",
                "generated_image",
                Some(id),
            )
        }));
    }
    if let Some(ids) = metadata
        .get("generation_context")
        .and_then(Value::as_object)
        .and_then(|context| context.get("reference_image_ids"))
        .and_then(Value::as_array)
    {
        refs.extend(ids.iter().filter_map(Value::as_i64).map(|id| {
            lineage_ref(
                &format!("generated_image:{id}"),
                "参考图片",
                "image",
                "generated_image",
                Some(id),
            )
        }));
    }
    refs
}

pub(super) fn manga_prompt_record(
    prompt: chapter_manga_prompt::Model,
    image: Option<&generated_image::Model>,
) -> AssetLineageRecord {
    let mut warnings = Vec::new();
    if prompt.generated_image_id.is_some() && image.is_none() {
        warnings.push("提示词已记录生成图片 ID，但未找到对应图片记录。".to_string());
    }
    AssetLineageRecord {
        id: format!("chapter_manga_prompt:{}", prompt.id),
        label: format!(
            "第 {} 章 第 {} 页 第 {} 格",
            prompt.chapter_id, prompt.page_number, prompt.panel_number
        ),
        asset_kind: "chapter_manga_prompt".to_string(),
        asset_id: Some(prompt.id),
        status: Some(prompt.status.clone()),
        origin: "llm_generated".to_string(),
        source_refs: vec![lineage_ref(
            &format!("chapter:{}", prompt.chapter_id),
            "章节正文",
            "text",
            "chapter_content",
            Some(prompt.chapter_id),
        )],
        prompt_sources: optional_prompt_sources(vec![
            prompt_source(
                &format!("chapter_manga_prompt:{}", prompt.id),
                "manga_image_prompt",
                Some(&prompt.image_prompt),
                Some(prompt.id),
                None,
            ),
            prompt_source(
                &format!("chapter_manga_dialogue:{}", prompt.id),
                "dialogue_text",
                prompt.dialogue_text.as_deref(),
                Some(prompt.id),
                None,
            ),
        ]),
        media_sources: image
            .map(|image| {
                vec![lineage_ref(
                    &format!("generated_image:{}", image.id),
                    "生成图片",
                    "image",
                    "generated_image",
                    Some(image.id),
                )]
            })
            .unwrap_or_default(),
        request_snapshot: image
            .and_then(|image| image.metadata_json.as_ref())
            .and_then(|value| value.get("provider_request_snapshot").cloned()),
        input_status: "missing".to_string(),
        generation_runs: Vec::new(),
        generation_inputs: Vec::new(),
        result: db_record_result_with_related(
            "chapter_manga_prompt",
            &prompt,
            image
                .map(|image| db_related_record("generated_image", image))
                .into_iter()
                .collect(),
        ),
        warnings,
    }
}

pub(super) fn video_prompt_sources(prompt: &chapter_video_prompt::Model) -> Vec<PromptSource> {
    optional_prompt_sources(vec![
        prompt_source(
            &format!("chapter_video_visual_prompt:{}", prompt.id),
            "video_visual_prompt",
            Some(&prompt.visual_prompt),
            Some(prompt.id),
            None,
        ),
        prompt_source(
            &format!("chapter_video_motion_prompt:{}", prompt.id),
            "video_motion_prompt",
            prompt.motion_prompt.as_deref(),
            Some(prompt.id),
            None,
        ),
        prompt_source(
            &format!("chapter_video_negative_prompt:{}", prompt.id),
            "video_negative_prompt",
            prompt.negative_prompt.as_deref(),
            Some(prompt.id),
            None,
        ),
    ])
}

pub(super) fn video_prompt_record(prompt: chapter_video_prompt::Model) -> AssetLineageRecord {
    AssetLineageRecord {
        id: format!("chapter_video_prompt:{}", prompt.id),
        label: prompt.title.clone().unwrap_or_else(|| {
            format!(
                "第 {} 章 镜头 {} {}",
                prompt.chapter_id, prompt.shot_number, prompt.prompt_type
            )
        }),
        asset_kind: video_prompt_asset_kind(&prompt.prompt_type).to_string(),
        asset_id: Some(prompt.id),
        status: Some(prompt.status.clone()),
        origin: "llm_generated".to_string(),
        source_refs: Vec::new(),
        prompt_sources: video_prompt_sources(&prompt),
        media_sources: prompt
            .reference_image_ids_json
            .as_ref()
            .and_then(Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(Value::as_i64)
                    .map(|id| {
                        lineage_ref(
                            &format!("generated_image:{id}"),
                            "参考图片",
                            "image",
                            "generated_image",
                            Some(id),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
        request_snapshot: prompt
            .metadata_json
            .as_ref()
            .and_then(|value| value.get("provider_request_snapshot").cloned()),
        input_status: "missing".to_string(),
        generation_runs: Vec::new(),
        generation_inputs: Vec::new(),
        result: db_record_result("chapter_video_prompt", &prompt),
        warnings: Vec::new(),
    }
}

pub(super) fn video_prompt_asset_kind(prompt_type: &str) -> &'static str {
    if prompt_type_str_in(prompt_type, VIDEO_SHOT_ANALYSIS_PROMPT_TYPES) {
        "video_shot_analysis"
    } else if prompt_type_str_in(prompt_type, VIDEO_SEGMENT_PLAN_PROMPT_TYPES) {
        "video_segment_plan"
    } else if prompt_type_str_in(prompt_type, VIDEO_FRAME_PROMPT_TYPES) {
        "video_frame_prompt"
    } else {
        "chapter_video_prompt"
    }
}

pub(super) fn generated_video_prompt_id(metadata: Option<&Value>) -> Option<i64> {
    let metadata = metadata?;
    metadata
        .get("prompt_id")
        .and_then(Value::as_i64)
        .or_else(|| {
            metadata
                .get("metadata")
                .and_then(Value::as_object)
                .and_then(|value| value.get("prompt_id"))
                .and_then(Value::as_i64)
        })
        .or_else(|| {
            metadata
                .get("generation_context")
                .and_then(Value::as_object)
                .and_then(|context| context.get("prompt_id"))
                .and_then(Value::as_i64)
        })
        .filter(|id| *id > 0)
}
