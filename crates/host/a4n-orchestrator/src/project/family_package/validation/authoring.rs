use super::super::ProjectFamilyPackage;
use a4n_domain::media_authoring::{
    AdaptationPlanV1, AdaptationPlanVersion, AdaptationTarget, AuthoringDocumentKind,
    AuthoringDocumentRecord, AuthoringDocumentStatus, AuthoringSourceRef,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_authoring_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    let mut receipts = BTreeSet::new();
    for receipt in &package.authoring_receipts {
        if receipt.kind.trim().is_empty()
            || receipt.old_id <= 0
            || receipt.new_id <= 0
            || !receipt.before_json.is_object()
            || !valid_hash(&receipt.after_sha256)
            || !package
                .projects
                .iter()
                .any(|project| project.id == receipt.project_id)
            || !receipts.insert((
                &receipt.kind,
                receipt.old_id,
                receipt.new_id,
                receipt.project_id,
            ))
        {
            return Err("项目包的创作迁移回执无效或重复".into());
        }
    }
    let mut records = BTreeMap::new();
    let mut versions = BTreeSet::new();
    let mut heads = BTreeSet::new();
    for row in &package.authoring_documents {
        let record = a4n_db::media_authoring::parse_family_transfer_record(row.clone())
            .map_err(|error| error.to_string())?;
        if record.id <= 0
            || record.revision <= 0
            || record.version <= 0
            || records.insert(record.id, record.clone()).is_some()
        {
            return Err(format!("创作文档身份或版本无效: {}", record.id));
        }
        let scope = (
            record.project_id,
            record.source_snapshot_id,
            record.source_chapter_id,
            record.kind.as_str(),
        );
        if !versions.insert((scope, record.version))
            || (record.status != AuthoringDocumentStatus::Superseded
                && !heads.insert((scope, record.status.as_str())))
        {
            return Err(format!("创作文档版本或活动状态重复: {}", record.id));
        }
    }
    for record in records.values() {
        let project = package
            .projects
            .iter()
            .find(|project| project.id == record.project_id)
            .ok_or("创作文档所属项目不在导入包内")?;
        let target = match project.project_kind.as_str() {
            "comic" => AdaptationTarget::Manga,
            "galgame" => AdaptationTarget::Galgame,
            _ => return Err("创作文档只能属于漫画或 Galgame 派生项目".into()),
        };
        if project.root_project_id != Some(package.root_project_id)
            || !package.source_snapshots.iter().any(|snapshot| {
                snapshot.id == record.source_snapshot_id
                    && snapshot.root_project_id == package.root_project_id
            })
            || record.inputs.sources.is_empty()
        {
            return Err(format!("创作文档 {} 的原作或基准快照无效", record.id));
        }
        if (record.kind == AuthoringDocumentKind::GalgameBranchPlan
            && record.source_chapter_id.is_some())
            || (record.kind != AuthoringDocumentKind::GalgameBranchPlan
                && (record.inputs.sources.len() != 1
                    || record.source_chapter_id
                        != Some(record.inputs.sources[0].source_chapter_id)
                    || record.source_snapshot_id != record.inputs.sources[0].source_snapshot_id))
        {
            return Err(format!("创作文档 {} 的章节范围无效", record.id));
        }
        let mut source_ids = BTreeSet::new();
        for source in &record.inputs.sources {
            validate_source(package, source)?;
            if !source_ids.insert(source.source_snapshot_chapter_id) {
                return Err("创作文档原文章节重复".into());
            }
        }
        let mut dependencies = BTreeSet::new();
        for dependency in &record.inputs.dependencies {
            let parent = records
                .get(&dependency.id)
                .ok_or("创作文档依赖不在导入包内")?;
            if parent.project_id != record.project_id
                || parent.kind != AuthoringDocumentKind::AdaptationPlan
                || parent.status == AuthoringDocumentStatus::Draft
                || parent.reference() != *dependency
                || !dependencies.insert(dependency.id)
            {
                return Err(format!("创作文档 {} 的依赖版本无效", record.id));
            }
        }
        if let Some(reference) = &record.inputs.video_reference {
            validate_source(package, &reference.source)?;
            if reference.script_id <= 0
                || reference.script_revision <= 0
                || !valid_hash(&reference.script_sha256)
                || reference.project_title.trim().is_empty()
                || record.kind != AuthoringDocumentKind::AdaptationPlan
            {
                return Err("冻结视频参考的身份无效".into());
            }
            if let Some(project) = reference
                .video_project_id
                .and_then(|id| package.projects.iter().find(|project| project.id == id))
                && (project.project_kind != "video"
                    || project.root_project_id != Some(package.root_project_id))
            {
                return Err("视频参考不属于同一原作".into());
            }
            // The captured version remains valid after its video is edited or
            // omitted from a single-project export.
            AdaptationPlanV1 {
                schema_version: AdaptationPlanVersion::V1,
                target,
                intention: "冻结视频参考".into(),
                decisions: Vec::new(),
                units: reference.units.clone(),
            }
            .validate(&source_spans(package, &reference.source)?)
            .map_err(|error| format!("冻结视频参考: {error}"))?;
        }
        match record.kind {
            AuthoringDocumentKind::AdaptationPlan => {
                if !record.inputs.dependencies.is_empty() {
                    return Err("改编计划不能依赖下游文档".into());
                }
                let plan: AdaptationPlanV1 = serde_json::from_value(record.document.clone())
                    .map_err(|error| error.to_string())?;
                if plan.target != target {
                    return Err("改编计划媒介与项目不一致".into());
                }
                plan.validate(&source_spans(package, &record.inputs.sources[0])?)?;
            }
            AuthoringDocumentKind::MangaPagePlan => {
                if target != AdaptationTarget::Manga || record.inputs.dependencies.len() != 1 {
                    return Err("分页分镜依赖无效".into());
                }
                let parent = &records[&record.inputs.dependencies[0].id];
                if parent.inputs.sources != record.inputs.sources {
                    return Err("分页分镜与改编计划原文不一致".into());
                }
                let adaptation = crate::media_authoring::PreparedAdaptation {
                    record: parent.clone(),
                    plan: serde_json::from_value(parent.document.clone())
                        .map_err(|error| error.to_string())?,
                };
                crate::manga::parse_manga_page_plan(&record.document, &adaptation, "导入分页分镜")
                    .map_err(|error| error.to_string())?;
            }
            AuthoringDocumentKind::GalgameBranchPlan => {
                if target != AdaptationTarget::Galgame
                    || record.inputs.dependencies.len() != record.inputs.sources.len()
                {
                    return Err("分支计划依赖无效".into());
                }
                validate_branches(package, record, &records)?;
            }
        }
    }
    Ok(())
}

fn validate_source(
    package: &ProjectFamilyPackage,
    source: &AuthoringSourceRef,
) -> Result<(), String> {
    if source.root_project_id != package.root_project_id
        || !package.source_snapshot_chapters.iter().any(|chapter| {
            chapter.id == source.source_snapshot_chapter_id
                && chapter.snapshot_id == source.source_snapshot_id
                && chapter.root_project_id == source.root_project_id
                && chapter.source_chapter_id == source.source_chapter_id
                && chapter.content_hash == source.content_sha256
                && a4n_domain::source_text_hash(&chapter.content) == source.content_sha256
        })
    {
        return Err("创作文档冻结原文与导入包不一致".into());
    }
    Ok(())
}

fn source_spans(
    package: &ProjectFamilyPackage,
    source: &AuthoringSourceRef,
) -> Result<BTreeSet<String>, String> {
    let chapter = package
        .source_snapshot_chapters
        .iter()
        .find(|chapter| chapter.id == source.source_snapshot_chapter_id)
        .ok_or("创作文档原文不存在")?;
    Ok(a4n_domain::source_spans::source_spans_for_content(
        chapter.source_chapter_id,
        chapter.chapter_number,
        &chapter.content,
    )
    .into_iter()
    .map(|span| span.id)
    .collect())
}

fn validate_branches(
    package: &ProjectFamilyPackage,
    record: &AuthoringDocumentRecord,
    records: &BTreeMap<i64, AuthoringDocumentRecord>,
) -> Result<(), String> {
    let mut context = crate::galgame::GalgamePipelinePromptContext {
        project_id: record.project_id,
        root_project_id: package.root_project_id,
        source_snapshot_id: record.source_snapshot_id,
        chapters: Vec::new(),
        source_spans: Vec::new(),
        speaker_profiles: Vec::new(),
        adaptations: Vec::new(),
    };
    for (source, dependency) in record
        .inputs
        .sources
        .iter()
        .zip(&record.inputs.dependencies)
    {
        let parent = &records[&dependency.id];
        if parent.inputs.sources != [source.clone()] {
            return Err("分支计划的原文与改编依赖不一致".into());
        }
        let chapter = package
            .source_snapshot_chapters
            .iter()
            .find(|chapter| chapter.id == source.source_snapshot_chapter_id)
            .ok_or("分支原文不存在")?;
        let anchors = package
            .text_anchors
            .iter()
            .filter(|anchor| anchor.source_snapshot_chapter_id == Some(chapter.id))
            .map(
                |anchor| a4n_db::galgame::GalgamePipelinePromptTextAnchorRecord {
                    id: anchor.id,
                    source_snapshot_chapter_id: anchor.source_snapshot_chapter_id,
                    paragraph_index: anchor.paragraph_index,
                    start_offset: anchor.start_offset,
                    end_offset: anchor.end_offset,
                    quote: anchor.quote.clone(),
                },
            )
            .collect::<Vec<_>>();
        let prompt_chapter = a4n_db::galgame::GalgamePipelinePromptChapterSourceRecord {
            id: chapter.id,
            source_chapter_id: chapter.source_chapter_id,
            chapter_number: chapter.chapter_number,
            title: chapter.title.clone(),
            content: chapter.content.clone(),
            summary: chapter.summary.clone(),
            content_hash: chapter.content_hash.clone(),
        };
        context.chapters.push(
            crate::galgame::galgame_prompt_chapter_from_snapshot(&prompt_chapter, &anchors)
                .map_err(|error| error.to_string())?,
        );
        context
            .adaptations
            .push(crate::galgame::GalgameChapterAdaptation {
                source_chapter_id: source.source_chapter_id,
                source_snapshot_chapter_id: source.source_snapshot_chapter_id,
                chapter_number: chapter.chapter_number,
                adaptation_plan_id: parent.id,
                adaptation_plan_revision: parent.revision,
                plan: serde_json::from_value(parent.document.clone())
                    .map_err(|error| error.to_string())?,
            });
    }
    let projection = crate::galgame::galgame_adaptation_projection(&context)
        .map_err(|error| error.to_string())?;
    let branches =
        crate::galgame::branch_design_from_value(record.document.clone(), "导入分支计划")
            .map_err(|error| error.to_string())?;
    crate::galgame::validate_galgame_branch_design(&branches, &projection)
        .map_err(|error| error.to_string())
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
