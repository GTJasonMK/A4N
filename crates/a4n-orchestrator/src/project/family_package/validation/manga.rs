use std::collections::{HashMap, HashSet};

use a4n_core::models::{
    ai::generated_image,
    chapter::{
        chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE},
        chapter_shooting_script, chapter_visual_plan,
    },
    character::character_portrait,
    project::{novel_project, source_snapshot_chapter},
};
use a4n_domain::{
    canvas_transfer_runtime_identity_key,
    manga::{MangaPageGenerationLayoutV2, MangaReferenceAssetKind},
    project::ProjectKind,
    shooting_script::{CHAPTER_SHOOTING_SCRIPT_V1, ChapterShootingScriptV1},
};

use crate::{
    manga::manga_page_generation_layout,
    project::{ProjectFamilyPackage, hash_text, sha256_bytes},
};

pub(super) fn validate_manga_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    let context = MangaManifestContext::new(package);
    validate_shooting_scripts(package, &context)?;
    validate_character_portraits(package, &context)?;
    validate_generated_images(package, &context)?;
    validate_manga_prompts(package, &context)
}

struct MangaManifestContext<'a> {
    projects: HashMap<i64, &'a novel_project::Model>,
    chapters: HashMap<i64, &'a a4n_core::models::chapter::chapter::Model>,
    snapshot_chapters: HashMap<i64, &'a source_snapshot_chapter::Model>,
    scripts: HashMap<i64, &'a chapter_shooting_script::Model>,
    visual_plans: HashMap<i64, &'a chapter_visual_plan::Model>,
    portraits: HashMap<i64, &'a character_portrait::Model>,
    images: HashMap<i64, &'a generated_image::Model>,
}

impl<'a> MangaManifestContext<'a> {
    fn new(package: &'a ProjectFamilyPackage) -> Self {
        Self {
            projects: package.projects.iter().map(|row| (row.id, row)).collect(),
            chapters: package
                .text_core
                .chapters
                .iter()
                .map(|row| (row.id, row))
                .collect(),
            snapshot_chapters: package
                .source_snapshot_chapters
                .iter()
                .map(|row| (row.id, row))
                .collect(),
            scripts: package
                .chapter_shooting_scripts
                .iter()
                .map(|row| (row.id, row))
                .collect(),
            visual_plans: package
                .chapter_visual_plans
                .iter()
                .map(|row| (row.id, row))
                .collect(),
            portraits: package
                .character_portraits
                .iter()
                .map(|row| (row.id, row))
                .collect(),
            images: package
                .generated_images
                .iter()
                .map(|row| (row.id, row))
                .collect(),
        }
    }
}

fn validate_shooting_scripts(
    package: &ProjectFamilyPackage,
    context: &MangaManifestContext<'_>,
) -> Result<(), String> {
    for script in &package.chapter_shooting_scripts {
        positive_id(script.id, "chapter_shooting_script.id")?;
        if script.workflow_run_id.is_some() {
            return Err(format!(
                "项目族导入包拍摄剧本不能携带 workflow_run_id: shooting_script_id={}",
                script.id
            ));
        }
        if script.root_project_id != package.root_project_id {
            return Err(format!(
                "项目族导入包拍摄剧本不属于根项目: shooting_script_id={}, root_project_id={}",
                script.id, script.root_project_id
            ));
        }
        let chapter = context
            .chapters
            .get(&script.source_chapter_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包拍摄剧本引用了不存在的源章节: shooting_script_id={}, source_chapter_id={}",
                    script.id, script.source_chapter_id
                )
            })?;
        if chapter.project_id != package.root_project_id {
            return Err(format!(
                "项目族导入包拍摄剧本源章节不属于根项目: shooting_script_id={}, source_chapter_id={}",
                script.id, script.source_chapter_id
            ));
        }
        if script.contract_version != CHAPTER_SHOOTING_SCRIPT_V1 || script.revision <= 0 {
            return Err(format!(
                "项目族导入包拍摄剧本合同身份无效: shooting_script_id={}, contract_version={}, revision={}",
                script.id, script.contract_version, script.revision
            ));
        }
        required_text(
            &script.source_revision_key,
            "拍摄剧本 source_revision_key",
            script.id,
        )?;
        validate_sha256(
            &script.source_content_sha256,
            "拍摄剧本 source_content_sha256",
            script.id,
        )?;
        validate_sha256(
            &script.input_fingerprint_sha256,
            "拍摄剧本 input_fingerprint_sha256",
            script.id,
        )?;
        validate_sha256(&script.script_sha256, "拍摄剧本 script_sha256", script.id)?;
        let typed_script: ChapterShootingScriptV1 =
            serde_json::from_value(script.script_json.clone()).map_err(|error| {
                format!(
                    "项目族导入包拍摄剧本 JSON 无效: shooting_script_id={}, error={error}",
                    script.id
                )
            })?;
        typed_script.validate().map_err(|error| {
            format!(
                "项目族导入包拍摄剧本内容无效: shooting_script_id={}, error={error}",
                script.id
            )
        })?;
        let script_bytes = serde_json::to_vec(&typed_script)
            .map_err(|error| format!("序列化项目族拍摄剧本失败: {error}"))?;
        if sha256_bytes(&script_bytes) != script.script_sha256 {
            return Err(format!(
                "项目族导入包拍摄剧本内容哈希不一致: shooting_script_id={}",
                script.id
            ));
        }
        let (Some(visual_plan_id), Some(visual_plan_version), Some(visual_plan_sha256)) = (
            script.visual_plan_id,
            script.visual_plan_version,
            script.visual_plan_sha256.as_deref(),
        ) else {
            return Err(format!(
                "项目族导入包拍摄剧本必须绑定 visual plan: shooting_script_id={}",
                script.id
            ));
        };
        let visual_plan = context
            .visual_plans
            .get(&visual_plan_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包拍摄剧本引用了不存在的 visual plan: shooting_script_id={}, visual_plan_id={visual_plan_id}",
                    script.id
                )
            })?;
        let typed_source = typed_script.visual_plan_source.as_ref().ok_or_else(|| {
            format!(
                "项目族导入包拍摄剧本 JSON 缺少 visual_plan_source: shooting_script_id={}",
                script.id
            )
        })?;
        if !matches!(visual_plan.status.as_str(), "confirmed" | "superseded")
            || visual_plan.root_project_id != script.root_project_id
            || visual_plan.source_chapter_id != script.source_chapter_id
            || visual_plan.source_snapshot_id != script.source_snapshot_id
            || visual_plan.source_snapshot_chapter_id != script.source_snapshot_chapter_id
            || visual_plan.source_content_sha256 != script.source_content_sha256
            || visual_plan.plan_version != visual_plan_version
            || visual_plan.plan_sha256 != visual_plan_sha256
            || typed_source.plan_id != visual_plan_id
            || typed_source.plan_version != visual_plan_version
            || typed_source.plan_sha256 != visual_plan_sha256
        {
            return Err(format!(
                "项目族导入包拍摄剧本 visual plan 身份不一致: shooting_script_id={}",
                script.id
            ));
        }
        match (script.source_snapshot_id, script.source_snapshot_chapter_id) {
            (None, None) => {
                if script.source_content_sha256 != hash_text(&chapter.content) {
                    return Err(format!(
                        "项目族导入包拍摄剧本源正文哈希与章节不一致: shooting_script_id={}",
                        script.id
                    ));
                }
            }
            (Some(snapshot_id), Some(snapshot_chapter_id)) => {
                let snapshot_chapter = context
                    .snapshot_chapters
                    .get(&snapshot_chapter_id)
                    .copied()
                    .ok_or_else(|| {
                        format!(
                            "项目族导入包拍摄剧本引用了不存在的源快照章节: shooting_script_id={}, source_snapshot_chapter_id={snapshot_chapter_id}",
                            script.id
                        )
                    })?;
                if snapshot_chapter.snapshot_id != snapshot_id
                    || snapshot_chapter.root_project_id != package.root_project_id
                    || snapshot_chapter.source_chapter_id != script.source_chapter_id
                    || snapshot_chapter.content_hash != script.source_content_sha256
                {
                    return Err(format!(
                        "项目族导入包拍摄剧本源快照章节身份不一致: shooting_script_id={}, source_snapshot_id={snapshot_id}, source_snapshot_chapter_id={snapshot_chapter_id}",
                        script.id
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "项目族导入包拍摄剧本 source_snapshot_id 与 source_snapshot_chapter_id 必须同时提供或同时为空: shooting_script_id={}",
                    script.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_character_portraits(
    package: &ProjectFamilyPackage,
    context: &MangaManifestContext<'_>,
) -> Result<(), String> {
    let profiles = package
        .text_core
        .character_profiles
        .iter()
        .map(|row| (row.id, row))
        .collect::<HashMap<_, _>>();
    for portrait in &package.character_portraits {
        positive_id(portrait.id, "character_portrait.id")?;
        require_packaged_project(context, portrait.project_id, "角色立绘", portrait.id)?;
        let profile = profiles.get(&portrait.profile_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包角色立绘引用了不存在的角色档案: portrait_id={}, profile_id={}",
                portrait.id, portrait.profile_id
            )
        })?;
        if profile.project_id != portrait.project_id {
            return Err(format!(
                "项目族导入包角色立绘与角色档案跨项目: portrait_id={}, profile_id={}",
                portrait.id, portrait.profile_id
            ));
        }
        required_text(&portrait.image_path, "角色立绘 image_path", portrait.id)?;
        required_text(&portrait.prompt, "角色立绘 prompt", portrait.id)?;
        required_text(
            &portrait.portrait_type,
            "角色立绘 portrait_type",
            portrait.id,
        )?;
    }
    Ok(())
}

fn validate_generated_images(
    package: &ProjectFamilyPackage,
    context: &MangaManifestContext<'_>,
) -> Result<(), String> {
    for image in &package.generated_images {
        positive_id(image.id, "generated_image.id")?;
        if let Some(key) = image
            .metadata_json
            .as_ref()
            .and_then(canvas_transfer_runtime_identity_key)
        {
            return Err(format!(
                "项目族导入包生成图片 metadata_json 不能携带运行身份 {key}: generated_image_id={}",
                image.id
            ));
        }
        require_packaged_project(context, image.project_id, "生成图片", image.id)?;
        if let Some(chapter_id) = image.chapter_id {
            let chapter = context.chapters.get(&chapter_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包生成图片引用了不存在的章节: generated_image_id={}, chapter_id={chapter_id}",
                    image.id
                )
            })?;
            if chapter.project_id != package.root_project_id {
                return Err(format!(
                    "项目族导入包生成图片章节不属于根项目: generated_image_id={}, chapter_id={chapter_id}",
                    image.id
                ));
            }
        }
        required_text(&image.image_type, "生成图片 image_type", image.id)?;
        required_text(&image.file_path, "生成图片 file_path", image.id)?;
        required_text(&image.prompt, "生成图片 prompt", image.id)?;
        required_text(&image.model, "生成图片 model", image.id)?;
        if image.width <= 0 || image.height <= 0 || image.file_size.is_some_and(|size| size < 0) {
            return Err(format!(
                "项目族导入包生成图片尺寸或文件大小无效: generated_image_id={}, width={}, height={}, file_size={:?}",
                image.id, image.width, image.height, image.file_size
            ));
        }
    }
    Ok(())
}

fn validate_manga_prompts(
    package: &ProjectFamilyPackage,
    context: &MangaManifestContext<'_>,
) -> Result<(), String> {
    let mut identities = HashSet::new();
    for prompt in &package.chapter_manga_prompts {
        positive_id(prompt.id, "chapter_manga_prompt.id")?;
        let project =
            require_packaged_project(context, prompt.project_id, "漫画页面提示词", prompt.id)?;
        if !matches!(
            ProjectKind::parse(&project.project_kind),
            Some(ProjectKind::Text | ProjectKind::Comic)
        ) {
            return Err(format!(
                "项目族导入包漫画页面提示词所属项目不是文本或漫画项目: prompt_id={}, project_id={}",
                prompt.id, prompt.project_id
            ));
        }
        let chapter = context
            .chapters
            .get(&prompt.chapter_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包漫画页面提示词引用了不存在的章节: prompt_id={}, chapter_id={}",
                    prompt.id, prompt.chapter_id
                )
            })?;
        if chapter.project_id != package.root_project_id {
            return Err(format!(
                "项目族导入包漫画页面提示词章节不属于根项目: prompt_id={}, chapter_id={}",
                prompt.id, prompt.chapter_id
            ));
        }
        if prompt.prompt_type != MANGA_PAGE_PROMPT_TYPE
            || !matches!(
                prompt.status.as_str(),
                "pending" | "generating" | "completed" | "failed" | "cancelled"
            )
        {
            return Err(format!(
                "项目族导入包漫画页面提示词合同字段无效: prompt_id={}, prompt_type={}, status={}",
                prompt.id, prompt.prompt_type, prompt.status
            ));
        }
        required_text(
            &prompt.image_prompt,
            "漫画页面提示词 image_prompt",
            prompt.id,
        )?;
        if !identities.insert((prompt.project_id, prompt.chapter_id, prompt.page_number)) {
            return Err(format!(
                "项目族导入包漫画页面提示词身份重复: project_id={}, chapter_id={}, page_number={}",
                prompt.project_id, prompt.chapter_id, prompt.page_number
            ));
        }
        let layout = manga_page_generation_layout(prompt)?;
        validate_manga_layout(package, prompt, project, &layout, context)?;
        if let Some(image_id) = prompt.generated_image_id {
            let image = context.images.get(&image_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包漫画页面提示词引用了不存在的生成图片: prompt_id={}, generated_image_id={image_id}",
                    prompt.id
                )
            })?;
            if image.project_id != prompt.project_id
                || image.chapter_id != Some(prompt.chapter_id)
                || image.image_type != "manga_page"
            {
                return Err(format!(
                    "项目族导入包漫画页面提示词生成图片身份不一致: prompt_id={}, generated_image_id={image_id}",
                    prompt.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_manga_layout(
    package: &ProjectFamilyPackage,
    prompt: &chapter_manga_prompt::Model,
    project: &novel_project::Model,
    layout: &MangaPageGenerationLayoutV2,
    context: &MangaManifestContext<'_>,
) -> Result<(), String> {
    let script = context
        .scripts
        .get(&layout.source.shooting_script_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包漫画 layout 引用了不存在的拍摄剧本: prompt_id={}, shooting_script_id={}",
                prompt.id, layout.source.shooting_script_id
            )
        })?;
    if layout.source.root_project_id != package.root_project_id
        || layout.source.source_chapter_id != prompt.chapter_id
        || layout.source.source_snapshot_id != script.source_snapshot_id
        || layout.source.source_snapshot_chapter_id != script.source_snapshot_chapter_id
        || layout.source.source_revision_key != script.source_revision_key
        || layout.source.content_sha256 != script.source_content_sha256
        || layout.source.shooting_script_id != script.id
        || layout.source.shooting_script_revision != script.revision
        || layout.source.shooting_script_sha256 != script.script_sha256
    {
        return Err(format!(
            "项目族导入包漫画 layout 来源与拍摄剧本身份不一致: prompt_id={}, shooting_script_id={}",
            prompt.id, script.id
        ));
    }
    match ProjectKind::parse(&project.project_kind) {
        Some(ProjectKind::Text)
            if layout.source.source_snapshot_id.is_some()
                || layout.source.source_snapshot_chapter_id.is_some() =>
        {
            return Err(format!(
                "项目族导入包文本项目漫画 layout 不能引用源快照: prompt_id={}",
                prompt.id
            ));
        }
        Some(ProjectKind::Comic)
            if layout.source.source_snapshot_id != project.source_snapshot_id =>
        {
            return Err(format!(
                "项目族导入包漫画 layout 源快照与漫画项目不一致: prompt_id={}, project_id={}",
                prompt.id, prompt.project_id
            ));
        }
        _ => {}
    }
    let mut names = HashSet::new();
    for binding in &layout.reference_bindings {
        let name = binding.character_name.trim();
        if name.is_empty() || !names.insert(name) {
            return Err(format!(
                "项目族导入包漫画 layout 角色参考绑定名称为空或重复: prompt_id={}",
                prompt.id
            ));
        }
        let Some(asset) = binding.bound_asset.as_ref() else {
            continue;
        };
        positive_id(asset.asset_id, "漫画 layout reference asset_id")?;
        match asset.asset_kind {
            MangaReferenceAssetKind::CharacterPortrait => {
                let portrait = context.portraits.get(&asset.asset_id).copied().ok_or_else(|| {
                    format!(
                        "项目族导入包漫画 layout 引用了不存在的角色立绘: prompt_id={}, portrait_id={}",
                        prompt.id, asset.asset_id
                    )
                })?;
                if ![prompt.project_id, package.root_project_id].contains(&portrait.project_id)
                    || portrait.image_path != asset.image_path
                {
                    return Err(format!(
                        "项目族导入包漫画 layout 角色立绘身份不一致: prompt_id={}, portrait_id={}",
                        prompt.id, asset.asset_id
                    ));
                }
            }
            MangaReferenceAssetKind::GeneratedImage => {
                let image = context.images.get(&asset.asset_id).copied().ok_or_else(|| {
                    format!(
                        "项目族导入包漫画 layout 引用了不存在的生成图片: prompt_id={}, generated_image_id={}",
                        prompt.id, asset.asset_id
                    )
                })?;
                if ![prompt.project_id, package.root_project_id].contains(&image.project_id)
                    || image.file_path != asset.image_path
                {
                    return Err(format!(
                        "项目族导入包漫画 layout 参考图片身份不一致: prompt_id={}, generated_image_id={}",
                        prompt.id, asset.asset_id
                    ));
                }
            }
        }
    }
    Ok(())
}

fn require_packaged_project<'a>(
    context: &'a MangaManifestContext<'a>,
    project_id: i64,
    label: &str,
    row_id: i64,
) -> Result<&'a novel_project::Model, String> {
    context.projects.get(&project_id).copied().ok_or_else(|| {
        format!("项目族导入包{label}引用了不存在的项目: row_id={row_id}, project_id={project_id}")
    })
}

fn positive_id(value: i64, label: &str) -> Result<(), String> {
    if value <= 0 {
        return Err(format!("项目族导入包 {label} 必须大于 0: {value}"));
    }
    Ok(())
}

fn required_text(value: &str, label: &str, row_id: i64) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("项目族导入包{label}不能为空: row_id={row_id}"));
    }
    Ok(())
}

fn validate_sha256(value: &str, label: &str, row_id: i64) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "项目族导入包{label}必须是 64 位十六进制: row_id={row_id}"
        ));
    }
    Ok(())
}
