use std::collections::HashMap;

use a4n_core::{
    models::{
        ai::{generated_image, generated_video},
        chapter::{chapter, chapter_shooting_script, chapter_video_prompt},
        project::novel_project,
    },
    video_execution::VideoShootingScriptIdentityV1,
};
use a4n_domain::canvas_transfer_runtime_identity_key;
use serde_json::Value;

use crate::project::{PROJECT_KIND_TEXT, PROJECT_KIND_VIDEO, ProjectFamilyPackage};

pub(super) fn validate_video_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    let context = VideoManifestContext::new(package);
    for prompt in &package.chapter_video_prompts {
        validate_video_prompt(package, prompt, &context)?;
    }
    for video in &package.generated_videos {
        validate_generated_video(package, video, &context)?;
    }
    Ok(())
}

struct VideoManifestContext<'a> {
    projects: HashMap<i64, &'a novel_project::Model>,
    chapters: HashMap<i64, &'a chapter::Model>,
    shooting_scripts: HashMap<i64, &'a chapter_shooting_script::Model>,
    images: HashMap<i64, &'a generated_image::Model>,
    prompts: HashMap<i64, &'a chapter_video_prompt::Model>,
    videos: HashMap<i64, &'a generated_video::Model>,
}

impl<'a> VideoManifestContext<'a> {
    fn new(package: &'a ProjectFamilyPackage) -> Self {
        Self {
            projects: package.projects.iter().map(|row| (row.id, row)).collect(),
            chapters: package
                .text_core
                .chapters
                .iter()
                .map(|row| (row.id, row))
                .collect(),
            shooting_scripts: package
                .chapter_shooting_scripts
                .iter()
                .map(|row| (row.id, row))
                .collect(),
            images: package
                .generated_images
                .iter()
                .map(|row| (row.id, row))
                .collect(),
            prompts: package
                .chapter_video_prompts
                .iter()
                .map(|row| (row.id, row))
                .collect(),
            videos: package
                .generated_videos
                .iter()
                .map(|row| (row.id, row))
                .collect(),
        }
    }
}

fn validate_video_prompt(
    package: &ProjectFamilyPackage,
    prompt: &chapter_video_prompt::Model,
    context: &VideoManifestContext<'_>,
) -> Result<(), String> {
    require_positive_id(prompt.id, "chapter_video_prompt.id")?;
    validate_video_chapter_project(
        package,
        prompt.project_id,
        prompt.chapter_id,
        "章节视频提示词",
        prompt.id,
        context,
    )?;
    match prompt.status.as_str() {
        chapter_video_prompt::STATUS_RUNNING => {
            return Err(format!(
                "项目族导入包不能包含运行中的章节视频提示词: chapter_video_prompt_id={}",
                prompt.id
            ));
        }
        chapter_video_prompt::STATUS_PENDING
        | chapter_video_prompt::STATUS_FAILED
        | chapter_video_prompt::STATUS_INTERRUPTED
        | chapter_video_prompt::STATUS_COMPLETED
        | chapter_video_prompt::STATUS_CANCELLED => {}
        _ => {
            return Err(format!(
                "项目族导入包章节视频提示词状态无效: chapter_video_prompt_id={}, status={}",
                prompt.id, prompt.status
            ));
        }
    }
    if prompt.timeline_item_id.is_some() {
        return Err(format!(
            "项目族导入包章节视频提示词不能携带 timeline_item_id: chapter_video_prompt_id={}",
            prompt.id
        ));
    }
    if let Some(key) = prompt
        .metadata_json
        .as_ref()
        .and_then(canvas_transfer_runtime_identity_key)
    {
        return Err(format!(
            "项目族导入包章节视频提示词 metadata_json 不能携带运行身份 {key}: chapter_video_prompt_id={}",
            prompt.id
        ));
    }

    for image_id in crate::video::reference_image_ids(prompt.reference_image_ids_json.as_ref())
        .map_err(|error| {
            format!(
                "项目族导入包章节视频提示词参考图片无效: chapter_video_prompt_id={}, error={error}",
                prompt.id
            )
        })?
    {
        let image = context.images.get(&image_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包章节视频提示词引用了不存在的参考图片: chapter_video_prompt_id={}, generated_image_id={image_id}",
                prompt.id
            )
        })?;
        if image.project_id != prompt.project_id && image.project_id != package.root_project_id {
            return Err(format!(
                "项目族导入包章节视频提示词参考图片不属于当前项目族上下文: chapter_video_prompt_id={}, generated_image_id={image_id}",
                prompt.id
            ));
        }
    }

    if let Some(video_id) = prompt.generated_video_id {
        let video = context.videos.get(&video_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包章节视频提示词引用了不存在的生成视频: chapter_video_prompt_id={}, generated_video_id={video_id}",
                prompt.id
            )
        })?;
        if video.project_id != prompt.project_id || video.chapter_id != Some(prompt.chapter_id) {
            return Err(format!(
                "项目族导入包章节视频提示词与生成视频上下文不一致: chapter_video_prompt_id={}, generated_video_id={video_id}",
                prompt.id
            ));
        }
        let linked_prompt_id = optional_positive_i64(
            video.metadata_json.as_ref(),
            &["prompt_id"],
            "generated_video.metadata_json.prompt_id",
        )?
        .ok_or_else(|| {
            format!(
                "项目族导入包章节视频提示词关联的生成视频缺少 prompt_id: chapter_video_prompt_id={}, generated_video_id={video_id}",
                prompt.id
            )
        })?;
        if linked_prompt_id != prompt.id {
            return Err(format!(
                "项目族导入包章节视频提示词与生成视频反向引用不一致: chapter_video_prompt_id={}, generated_video_id={video_id}, linked_prompt_id={linked_prompt_id}",
                prompt.id
            ));
        }
    }

    validate_prompt_metadata_references(
        prompt.metadata_json.as_ref(),
        prompt.project_id,
        Some(prompt.chapter_id),
        "章节视频提示词",
        prompt.id,
        context,
    )
}

fn validate_generated_video(
    package: &ProjectFamilyPackage,
    video: &generated_video::Model,
    context: &VideoManifestContext<'_>,
) -> Result<(), String> {
    require_positive_id(video.id, "generated_video.id")?;
    let project = context
        .projects
        .get(&video.project_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包生成视频所属项目不存在: generated_video_id={}, project_id={}",
                video.id, video.project_id
            )
        })?;
    if project.id != package.root_project_id
        && project.root_project_id != Some(package.root_project_id)
    {
        return Err(format!(
            "项目族导入包生成视频所属项目不在当前项目族: generated_video_id={}, project_id={}",
            video.id, video.project_id
        ));
    }
    if let Some(chapter_id) = video.chapter_id {
        validate_video_chapter_project(
            package,
            video.project_id,
            chapter_id,
            "生成视频",
            video.id,
            context,
        )?;
    }
    if !generated_video::is_terminal_status(&video.status) {
        return Err(format!(
            "项目族导入包只能包含终态生成视频: generated_video_id={}, status={}",
            video.id, video.status
        ));
    }
    if video.timeline_item_id.is_some() {
        return Err(format!(
            "项目族导入包生成视频不能携带 timeline_item_id: generated_video_id={}",
            video.id
        ));
    }
    if video.provider_job_id.is_some() {
        return Err(format!(
            "项目族导入包生成视频不能携带 provider_job_id: generated_video_id={}",
            video.id
        ));
    }
    if let Some(key) = video
        .metadata_json
        .as_ref()
        .and_then(canvas_transfer_runtime_identity_key)
    {
        return Err(format!(
            "项目族导入包生成视频 metadata_json 不能携带运行身份 {key}: generated_video_id={}",
            video.id
        ));
    }

    if let Some(source_video_id) = video.source_video_id {
        let source = context.videos.get(&source_video_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包生成视频引用了不存在的源视频: generated_video_id={}, source_video_id={source_video_id}",
                video.id
            )
        })?;
        if source.id == video.id || source.project_id != video.project_id {
            return Err(format!(
                "项目族导入包生成视频源视频上下文无效: generated_video_id={}, source_video_id={source_video_id}",
                video.id
            ));
        }
    }
    if let Some(source_image_id) = video.source_image_id {
        let source = context.images.get(&source_image_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包生成视频引用了不存在的源图片: generated_video_id={}, source_image_id={source_image_id}",
                video.id
            )
        })?;
        if source.project_id != video.project_id {
            return Err(format!(
                "项目族导入包生成视频源图片与视频跨项目: generated_video_id={}, source_image_id={source_image_id}",
                video.id
            ));
        }
    }

    let chapter_id = video.chapter_id;
    if let Some(prompt_id) = optional_positive_i64(
        video.metadata_json.as_ref(),
        &["prompt_id"],
        "generated_video.metadata_json.prompt_id",
    )? {
        let prompt_chapter_id = chapter_id.ok_or_else(|| {
            format!(
                "项目族导入包生成视频包含 prompt_id 但缺少 chapter_id: generated_video_id={}",
                video.id
            )
        })?;
        let prompt = require_prompt_reference(
            prompt_id,
            video.project_id,
            Some(prompt_chapter_id),
            None,
            "生成视频 prompt_id",
            video.id,
            context,
        )?;
        if prompt.generated_video_id.is_some_and(|id| id != video.id)
            || (video.status == generated_video::STATUS_COMPLETED
                && prompt.generated_video_id != Some(video.id))
        {
            return Err(format!(
                "项目族导入包生成视频与提示词反向引用不一致: generated_video_id={}, chapter_video_prompt_id={prompt_id}",
                video.id
            ));
        }
    }
    validate_prompt_metadata_references(
        video.metadata_json.as_ref(),
        video.project_id,
        chapter_id,
        "生成视频",
        video.id,
        context,
    )
}

fn validate_video_chapter_project(
    package: &ProjectFamilyPackage,
    project_id: i64,
    chapter_id: i64,
    label: &str,
    record_id: i64,
    context: &VideoManifestContext<'_>,
) -> Result<(), String> {
    let project = context.projects.get(&project_id).copied().ok_or_else(|| {
        format!("项目族导入包{label}所属项目不存在: record_id={record_id}, project_id={project_id}")
    })?;
    let chapter = context.chapters.get(&chapter_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包{label}引用了不存在的章节: record_id={record_id}, chapter_id={chapter_id}"
        )
    })?;
    let valid = match project.project_kind.as_str() {
        PROJECT_KIND_TEXT => {
            project.id == package.root_project_id && chapter.project_id == project.id
        }
        PROJECT_KIND_VIDEO => {
            project.root_project_id == Some(package.root_project_id)
                && chapter.project_id == package.root_project_id
        }
        _ => false,
    };
    if !valid {
        return Err(format!(
            "项目族导入包{label}的项目与章节不属于同一视频项目族上下文: record_id={record_id}, project_id={project_id}, chapter_id={chapter_id}"
        ));
    }
    Ok(())
}

fn validate_prompt_metadata_references(
    metadata: Option<&Value>,
    project_id: i64,
    chapter_id: Option<i64>,
    owner_label: &str,
    owner_id: i64,
    context: &VideoManifestContext<'_>,
) -> Result<(), String> {
    for path in [
        &["segment_plan_prompt_id"][..],
        &["video_execution_identity", "segment_plan_prompt_id"][..],
    ] {
        if let Some(prompt_id) = optional_positive_i64(metadata, path, &path.join("."))? {
            let chapter_id = chapter_id.ok_or_else(|| {
                format!(
                    "项目族导入包{owner_label}包含章节视频提示词引用但缺少 chapter_id: owner_id={owner_id}"
                )
            })?;
            require_prompt_reference(
                prompt_id,
                project_id,
                Some(chapter_id),
                Some(chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN),
                owner_label,
                owner_id,
                context,
            )?;
        }
    }
    validate_shooting_script_identity(metadata, chapter_id, owner_label, owner_id, context)
}

fn require_prompt_reference<'a>(
    prompt_id: i64,
    project_id: i64,
    chapter_id: Option<i64>,
    expected_type: Option<&str>,
    owner_label: &str,
    owner_id: i64,
    context: &VideoManifestContext<'a>,
) -> Result<&'a chapter_video_prompt::Model, String> {
    let prompt = context.prompts.get(&prompt_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包{owner_label}引用了不存在的章节视频提示词: owner_id={owner_id}, chapter_video_prompt_id={prompt_id}"
        )
    })?;
    if prompt.project_id != project_id || chapter_id.is_some_and(|id| id != prompt.chapter_id) {
        return Err(format!(
            "项目族导入包{owner_label}引用的章节视频提示词上下文不一致: owner_id={owner_id}, chapter_video_prompt_id={prompt_id}"
        ));
    }
    if expected_type.is_some_and(|expected| prompt.prompt_type != expected) {
        return Err(format!(
            "项目族导入包{owner_label}引用的片段规划提示词类型无效: owner_id={owner_id}, chapter_video_prompt_id={prompt_id}, prompt_type={}",
            prompt.prompt_type
        ));
    }
    Ok(prompt)
}

fn validate_shooting_script_identity(
    metadata: Option<&Value>,
    chapter_id: Option<i64>,
    owner_label: &str,
    owner_id: i64,
    context: &VideoManifestContext<'_>,
) -> Result<(), String> {
    let Some(metadata) = metadata else {
        return Ok(());
    };
    let Some(execution_identity) = metadata.get("video_execution_identity") else {
        return Ok(());
    };
    let execution_identity = execution_identity.as_object().ok_or_else(|| {
        format!(
            "项目族导入包{owner_label} video_execution_identity 必须是对象: owner_id={owner_id}"
        )
    })?;
    let Some(identity) = execution_identity.get("shooting_script_identity") else {
        return Ok(());
    };
    let identity = serde_json::from_value::<VideoShootingScriptIdentityV1>(identity.clone())
        .map_err(|error| {
            format!(
                "项目族导入包{owner_label} shooting_script_identity 无效: owner_id={owner_id}, error={error}"
            )
        })?;
    let chapter_id = chapter_id.ok_or_else(|| {
        format!("项目族导入包{owner_label}包含拍摄剧本身份但缺少 chapter_id: owner_id={owner_id}")
    })?;
    let script = context
        .shooting_scripts
        .get(&identity.record_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包{owner_label}引用了不存在的拍摄剧本: owner_id={owner_id}, shooting_script_id={}",
                identity.record_id
            )
        })?;
    if script.source_chapter_id != chapter_id
        || script.revision != identity.revision
        || script.source_revision_key != identity.source_revision_key
        || script.source_content_sha256 != identity.source_content_sha256
        || script.script_sha256 != identity.script_sha256
    {
        return Err(format!(
            "项目族导入包{owner_label}拍摄剧本身份不一致: owner_id={owner_id}, shooting_script_id={}",
            identity.record_id
        ));
    }
    Ok(())
}

fn optional_positive_i64(
    value: Option<&Value>,
    path: &[&str],
    label: &str,
) -> Result<Option<i64>, String> {
    let Some(mut value) = value else {
        return Ok(None);
    };
    for key in path {
        let object = value
            .as_object()
            .ok_or_else(|| format!("{label} 的父级必须是 JSON 对象"))?;
        let Some(next) = object.get(*key) else {
            return Ok(None);
        };
        value = next;
    }
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_i64()
        .filter(|id| *id > 0)
        .map(Some)
        .ok_or_else(|| format!("{label} 必须是正整数"))
}

fn require_positive_id(id: i64, label: &str) -> Result<(), String> {
    if id > 0 {
        Ok(())
    } else {
        Err(format!("{label} 必须是正整数"))
    }
}
