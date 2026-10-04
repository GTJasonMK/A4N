use std::collections::{HashMap, HashSet};

use a4n_domain::chapter_visual_plan::{CHAPTER_VISUAL_PLAN_V3, ChapterVisualPlanV3};

use super::super::{ProjectFamilyPackage, hash_text};

pub(super) fn validate_visual_plan_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    let chapters = package
        .text_core
        .chapters
        .iter()
        .map(|row| (row.id, row))
        .collect::<HashMap<_, _>>();
    let snapshot_chapters = package
        .source_snapshot_chapters
        .iter()
        .map(|row| (row.id, row))
        .collect::<HashMap<_, _>>();
    let mut versions = HashSet::new();
    let mut mutable_heads = HashSet::new();

    for plan in &package.chapter_visual_plans {
        validate_plan_record(package, plan, &chapters, &snapshot_chapters)?;
        if !versions.insert((
            plan.project_id,
            plan.source_chapter_id,
            plan.source_snapshot_id,
            plan.plan_version,
        )) {
            return Err(format!(
                "项目族导入包章节视觉编排版本重复: source_chapter_id={}, plan_version={}",
                plan.source_chapter_id, plan.plan_version
            ));
        }
        if matches!(plan.status.as_str(), "draft" | "confirmed")
            && !mutable_heads.insert((
                plan.project_id,
                plan.source_chapter_id,
                plan.source_snapshot_id,
                plan.source_snapshot_chapter_id,
                plan.status.as_str(),
            ))
        {
            return Err(format!(
                "项目族导入包章节视觉编排活动状态重复: source_chapter_id={}, source_snapshot_id={:?}, source_snapshot_chapter_id={:?}, status={}",
                plan.source_chapter_id,
                plan.source_snapshot_id,
                plan.source_snapshot_chapter_id,
                plan.status
            ));
        }
    }

    Ok(())
}

fn validate_plan_record(
    package: &ProjectFamilyPackage,
    plan: &a4n_domain::project::ProjectFamilyVisualPlanRecord,
    chapters: &HashMap<i64, &a4n_domain::project::ProjectFamilyChapterRecord>,
    snapshot_chapters: &HashMap<
        i64,
        &a4n_domain::project::ProjectFamilySourceSnapshotChapterRecord,
    >,
) -> Result<(), String> {
    if plan.id <= 0
        || plan.root_project_id != package.root_project_id
        || !package.projects.iter().any(|project| {
            project.id == plan.project_id
                && project.project_kind == "video"
                && project.root_project_id == Some(plan.root_project_id)
        })
    {
        return Err(format!(
            "项目族导入包章节视觉编排身份无效: visual_plan_id={}",
            plan.id
        ));
    }
    let chapter = chapters.get(&plan.source_chapter_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包章节视觉编排引用了不存在的章节: visual_plan_id={}, source_chapter_id={}",
            plan.id, plan.source_chapter_id
        )
    })?;
    if chapter.project_id != package.root_project_id
        || plan.contract_version != CHAPTER_VISUAL_PLAN_V3
        || plan.plan_version <= 0
        || plan.revision <= 0
        || !matches!(
            plan.status.as_str(),
            "draft" | "confirmed" | "superseded" | "stale"
        )
        || !matches!(plan.authoring_mode.as_str(), "collaborative" | "manual")
        || plan.author_user_id <= 0
        || (plan.status == "confirmed" && plan.confirmed_at.is_none())
    {
        return Err(format!(
            "项目族导入包章节视觉编排合同字段无效: visual_plan_id={}",
            plan.id
        ));
    }
    validate_sha256(
        &plan.source_content_sha256,
        "source_content_sha256",
        plan.id,
    )?;
    validate_sha256(&plan.plan_sha256, "plan_sha256", plan.id)?;

    let (source_content, expected_revision_key) = match (
        plan.source_snapshot_id,
        plan.source_snapshot_chapter_id,
    ) {
        (Some(snapshot_id), Some(snapshot_chapter_id)) => {
            let snapshot_chapter = snapshot_chapters
                .get(&snapshot_chapter_id)
                .copied()
                .ok_or_else(|| {
                    format!(
                        "项目族导入包章节视觉编排引用了不存在的源快照章节: visual_plan_id={}, source_snapshot_chapter_id={snapshot_chapter_id}",
                        plan.id
                    )
                })?;
            if snapshot_chapter.snapshot_id != snapshot_id
                || snapshot_chapter.root_project_id != package.root_project_id
                || snapshot_chapter.source_chapter_id != plan.source_chapter_id
            {
                return Err(format!(
                    "项目族导入包章节视觉编排源快照身份不一致: visual_plan_id={}",
                    plan.id
                ));
            }
            (
                snapshot_chapter.content.as_str(),
                format!(
                    "snapshot:{snapshot_id}:chapter:{snapshot_chapter_id}:{}",
                    plan.source_content_sha256
                ),
            )
        }
        _ => {
            return Err(format!(
                "项目族导入包章节视觉编排源快照身份不完整: visual_plan_id={}",
                plan.id
            ));
        }
    };
    if plan.source_content_sha256 != hash_text(source_content)
        || plan.source_revision_key != expected_revision_key
    {
        return Err(format!(
            "项目族导入包章节视觉编排来源哈希或 revision key 不一致: visual_plan_id={}",
            plan.id
        ));
    }

    let typed: ChapterVisualPlanV3 = serde_json::from_value(plan.plan_json.clone())
        .map_err(|error| format!("项目族导入包章节视觉编排 JSON 无效: {}: {error}", plan.id))?;
    let validation = if plan.status == "confirmed" {
        typed.validate_confirmed()
    } else {
        typed.validate_draft()
    };
    validation
        .map_err(|error| format!("项目族导入包章节视觉编排内容无效: {}: {error}", plan.id))?;
    let image_ids = package
        .generated_images
        .iter()
        .filter(|image| {
            image.project_id == package.root_project_id || image.project_id == plan.project_id
        })
        .map(|image| image.id)
        .collect::<HashSet<_>>();
    let scene_ids = package
        .text_core
        .scene_profiles
        .iter()
        .filter(|scene| {
            scene.project_id == package.root_project_id || scene.project_id == plan.project_id
        })
        .map(|scene| scene.id)
        .collect::<HashSet<_>>();
    if typed
        .referenced_image_ids()
        .iter()
        .any(|id| !image_ids.contains(id))
        || typed
            .referenced_scene_ids()
            .iter()
            .any(|id| !scene_ids.contains(id))
    {
        return Err(format!("导演编排 {} 引用了导入包外的图片或场景", plan.id));
    }
    let canonical = typed
        .canonical_json()
        .map_err(|error| format!("序列化项目族章节视觉编排失败: {error}"))?;
    let plan_sha256 = a4n_db::generation::canonical_json_sha256(&typed)
        .map_err(|error| format!("计算项目族章节视觉编排哈希失败: {error}"))?;
    if canonical != plan.plan_json || plan_sha256 != plan.plan_sha256 {
        return Err(format!(
            "项目族导入包章节视觉编排规范 JSON 或哈希不一致: visual_plan_id={}",
            plan.id
        ));
    }
    Ok(())
}

fn validate_sha256(value: &str, label: &str, id: i64) -> Result<(), String> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!(
            "项目族导入包章节视觉编排 {label} 无效: visual_plan_id={id}"
        ))
    }
}
