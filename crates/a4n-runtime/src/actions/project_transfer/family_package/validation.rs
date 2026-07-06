use super::*;

mod source_snapshots;

use self::source_snapshots::{parse_chapter_scope_json, validate_source_snapshot_manifest};

const GALGAME_CG_IMPORTANCE: &[&str] = &["normal", "high", "critical"];

macro_rules! ensure_package {
    ($condition:expr, $($arg:tt)*) => {
        if !$condition {
            return Err(format!($($arg)*));
        }
    };
}

pub(super) fn validate_package_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    if package.version != PROJECT_FAMILY_PACKAGE_VERSION {
        return Err(format!("项目族导入包版本无效: {}", package.version));
    }
    if package.mode != "project_family" && package.mode != "derived_project" {
        return Err(format!("项目族导入包模式无效: {}", package.mode));
    }
    let root = package
        .projects
        .iter()
        .find(|project| project.id == package.root_project_id)
        .ok_or_else(|| format!("项目族导入包缺少根文本项目: {}", package.root_project_id))?;
    if root.project_kind != PROJECT_KIND_TEXT {
        return Err("项目族导入包根项目必须是 text".to_string());
    }
    if root.root_project_id.is_some()
        || root.parent_project_id.is_some()
        || root.source_snapshot_id.is_some()
    {
        return Err("项目族导入包根文本项目不能包含派生项目引用字段".to_string());
    }
    let mut project_ids = HashSet::new();
    for project in &package.projects {
        if !project_ids.insert(project.id) {
            return Err(format!("项目族导入包项目 ID 重复: {}", project.id));
        }
        normalize_project_type(&project.project_type).map_err(|err| err.to_string())?;
        normalize_project_kind(&project.project_kind).map_err(|err| err.to_string())?;
        StateValidator::parse_status(&project.status).map_err(|err| err.to_string())?;
        if !is_valid_source_sync_status(&project.source_sync_status) {
            return Err(format!(
                "项目族导入包项目源同步状态无效: project_id={}, source_sync_status={}",
                project.id, project.source_sync_status
            ));
        }
        if project.project_kind == PROJECT_KIND_TEXT && project.id != package.root_project_id {
            return Err(format!("项目族导入包包含非根文本项目: {}", project.id));
        }
        if project.project_kind == PROJECT_KIND_TEXT {
            if project.source_sync_status != SOURCE_SYNC_CURRENT {
                return Err(format!(
                    "项目族导入包文本项目源同步状态必须是 current: project_id={}, source_sync_status={}",
                    project.id, project.source_sync_status
                ));
            }
            if project.derived_project_status.is_some() {
                return Err(format!(
                    "项目族导入包文本项目不能包含派生项目状态: project_id={}",
                    project.id
                ));
            }
        }
        if project.project_kind != PROJECT_KIND_TEXT {
            normalize_derived_project_kind(&project.project_kind).map_err(|err| err.to_string())?;
            if project.root_project_id != Some(package.root_project_id) {
                return Err(format!("派生项目不属于导入包根项目: {}", project.id));
            }
            if project.parent_project_id != Some(package.root_project_id) {
                return Err(format!("派生项目 parent_project_id 无效: {}", project.id));
            }
            if project.source_snapshot_id.is_none() {
                return Err(format!("派生项目缺少 source_snapshot_id: {}", project.id));
            }
            match project.derived_project_status.as_deref() {
                Some(status) if is_valid_derived_project_status(status) => {}
                Some(status) => {
                    return Err(format!(
                        "项目族导入包派生项目状态无效: project_id={}, derived_project_status={status}",
                        project.id
                    ));
                }
                None => {
                    return Err(format!(
                        "项目族导入包派生项目缺少 derived_project_status: project_id={}",
                        project.id
                    ));
                }
            }
        }
    }
    validate_package_row_id_uniqueness(package)?;
    validate_unique_ids(
        package
            .project_derivations
            .iter()
            .map(|record| record.derived_project_id),
        "project_derivation.derived_project_id",
    )?;
    let snapshot_ids = package
        .source_snapshots
        .iter()
        .map(|snapshot| snapshot.id)
        .collect::<HashSet<_>>();
    validate_source_snapshot_manifest(package)?;
    for project in &package.projects {
        if let Some(snapshot_id) = project.source_snapshot_id
            && !snapshot_ids.contains(&snapshot_id)
        {
            return Err(format!(
                "派生项目引用了导入包不存在的源文本快照: project_id={}, snapshot_id={}",
                project.id, snapshot_id
            ));
        }
    }
    let derivation_project_ids = package
        .project_derivations
        .iter()
        .map(|record| record.derived_project_id)
        .collect::<HashSet<_>>();
    for project in &package.projects {
        if project.project_kind != PROJECT_KIND_TEXT
            && !derivation_project_ids.contains(&project.id)
        {
            return Err(format!("派生项目缺少 project_derivation: {}", project.id));
        }
    }
    for derivation in &package.project_derivations {
        if derivation.root_project_id != package.root_project_id {
            return Err(format!("派生关系根项目无效: {}", derivation.id));
        }
        parse_chapter_scope_json(
            &derivation.chapter_scope_json,
            "派生关系 chapter_scope_json",
        )?;
        if !derivation.creation_options_json.is_object() {
            return Err(format!(
                "项目族导入包派生关系 creation_options_json 必须是 JSON 对象: project_derivation_id={}",
                derivation.id
            ));
        }
        let derived = package
            .projects
            .iter()
            .find(|project| project.id == derivation.derived_project_id)
            .ok_or_else(|| format!("派生关系引用了不存在的派生项目: {}", derivation.id))?;
        if derived.project_kind == PROJECT_KIND_TEXT {
            return Err(format!("派生关系引用了文本项目: {}", derived.id));
        }
        if derivation.derived_kind != derived.project_kind {
            return Err(format!(
                "派生关系类型与派生项目类型不一致: {}",
                derivation.id
            ));
        }
        let snapshot = package
            .source_snapshots
            .iter()
            .find(|snapshot| snapshot.id == derivation.source_snapshot_id)
            .ok_or_else(|| {
                format!(
                    "派生关系引用了导入包不存在的源文本快照: {}",
                    derivation.source_snapshot_id
                )
            })?;
        if derivation.chapter_scope_json != snapshot.chapter_scope_json {
            return Err(format!(
                "项目族导入包派生关系章节范围与源文本快照不一致: project_derivation_id={}, source_snapshot_id={}",
                derivation.id, snapshot.id
            ));
        }
    }
    validate_text_core_part_outline_ranges(package)?;
    validate_text_core_chapter_outline_statuses(package)?;
    validate_text_core_chapter_outline_part_references(package)?;
    validate_text_core_chapter_outline_references(package)?;
    validate_text_anchor_manifest(package)?;
    validate_profile_evidence_manifest(package)?;
    validate_galgame_manifest(package)?;
    validate_generation_manifest(package)?;
    validate_source_sync_manifest(package)?;
    Ok(())
}

fn validate_package_row_id_uniqueness(package: &ProjectFamilyPackage) -> Result<(), String> {
    macro_rules! validate_id_groups {
        ($(($records:expr, $field:ident, $label:literal)),+ $(,)?) => {
            $(
                validate_unique_ids(($records).iter().map(|record| record.$field), $label)?;
            )+
        };
    }

    if let Some(blueprint) = &package.text_core.blueprint {
        validate_unique_ids(std::iter::once(blueprint.id), "text_core.blueprint.id")?;
    }
    validate_id_groups!(
        (
            &package.text_core.blueprint_versions,
            id,
            "text_core.blueprint_versions.id"
        ),
        (
            &package.text_core.blueprint_characters,
            id,
            "text_core.blueprint_characters.id"
        ),
        (
            &package.text_core.blueprint_relationships,
            id,
            "text_core.blueprint_relationships.id"
        ),
        (
            &package.text_core.part_outlines,
            id,
            "text_core.part_outlines.id"
        ),
        (
            &package.text_core.chapter_outlines,
            id,
            "text_core.chapter_outlines.id"
        ),
        (&package.text_core.chapters, id, "text_core.chapters.id"),
        (
            &package.text_core.character_profiles,
            id,
            "text_core.character_profiles.id"
        ),
        (
            &package.text_core.object_profiles,
            id,
            "text_core.object_profiles.id"
        ),
        (
            &package.text_core.scene_profiles,
            id,
            "text_core.scene_profiles.id"
        ),
        (
            &package.text_core.scene_occurrences,
            id,
            "text_core.scene_occurrences.id"
        ),
        (
            &package.text_core.scene_transitions,
            id,
            "text_core.scene_transitions.id"
        ),
        (
            &package.text_core.scene_entity_states,
            id,
            "text_core.scene_entity_states.id"
        ),
        (&package.project_derivations, id, "project_derivations.id"),
        (&package.source_snapshots, id, "source_snapshots.id"),
        (
            &package.source_snapshot_chapters,
            id,
            "source_snapshot_chapters.id"
        ),
        (
            &package.source_snapshot_outlines,
            id,
            "source_snapshot_outlines.id"
        ),
        (
            &package.source_snapshot_profiles,
            id,
            "source_snapshot_profiles.id"
        ),
        (&package.text_anchors, id, "text_anchors.id"),
        (&package.profile_evidence, id, "profile_evidence.id"),
        (&package.galgame_scripts, id, "galgame_scripts.id"),
        (&package.galgame_scenes, id, "galgame_scenes.id"),
        (&package.galgame_lines, id, "galgame_lines.id"),
        (&package.galgame_choices, id, "galgame_choices.id"),
        (&package.generation_specs, id, "generation_specs.id"),
    );

    validate_unique_key_strings(
        package.generation_spec_text_anchors.iter().map(|record| {
            format!(
                "{}/{}/{}",
                record.generation_spec_id, record.text_anchor_id, record.role
            )
        }),
        "generation_spec_text_anchors(generation_spec_id,text_anchor_id,role)",
    )?;
    validate_unique_key_strings(
        package.generation_spec_entity_refs.iter().map(|record| {
            format!(
                "{}/{}/{}/{}",
                record.generation_spec_id, record.entity_kind, record.entity_id, record.role
            )
        }),
        "generation_spec_entity_refs(generation_spec_id,entity_kind,entity_id,role)",
    )?;
    validate_unique_key_strings(
        package.generation_spec_media_refs.iter().map(|record| {
            format!(
                "{}/{}/{}",
                record.generation_spec_id, record.media_asset_id, record.role
            )
        }),
        "generation_spec_media_refs(generation_spec_id,media_asset_id,role)",
    )?;

    validate_id_groups!(
        (&package.prompt_artifacts, id, "prompt_artifacts.id"),
        (&package.media_assets, id, "media_assets.id"),
        (
            &package.media_provenance,
            media_asset_id,
            "media_provenance.media_asset_id"
        ),
        (&package.media_bindings, id, "media_bindings.id"),
        (&package.video_segments, id, "video_segments.id"),
        (&package.assembly_timelines, id, "assembly_timelines.id"),
        (&package.assembly_clips, id, "assembly_clips.id"),
        (&package.source_sync_runs, id, "source_sync_runs.id"),
        (
            &package.source_sync_conflicts,
            id,
            "source_sync_conflicts.id"
        ),
        (
            &package.source_sync_decisions,
            id,
            "source_sync_decisions.id"
        ),
        (
            &package.source_sync_decisions,
            conflict_id,
            "source_sync_decisions.conflict_id"
        ),
    );
    Ok(())
}

pub(super) fn snapshot_profile_key(kind: &str, id: i64) -> String {
    let snapshot_kind = match kind {
        "character_profile" => "character",
        value => value,
    };
    format!("{snapshot_kind}/{id}")
}

fn validate_unique_key_strings(
    keys: impl Iterator<Item = String>,
    label: &str,
) -> Result<(), String> {
    let mut seen = HashSet::new();
    for key in keys {
        if !seen.insert(key.clone()) {
            return Err(format!("导入包包含重复 {label}: {key}"));
        }
    }
    Ok(())
}

fn validate_text_core_part_outline_ranges(package: &ProjectFamilyPackage) -> Result<(), String> {
    let mut ranges = package
        .text_core
        .part_outlines
        .iter()
        .map(|part| {
            (
                part.start_chapter,
                part.end_chapter,
                part.part_number,
                part.id,
            )
        })
        .collect::<Vec<_>>();
    ranges.sort_by_key(|(start_chapter, end_chapter, part_number, id)| {
        (*start_chapter, *end_chapter, *part_number, *id)
    });
    for pair in ranges.windows(2) {
        let (prev_start, prev_end, prev_part, prev_id) = pair[0];
        let (start, end, part, id) = pair[1];
        if start <= prev_end {
            return Err(format!(
                "项目族导入包分部章节范围重叠: part_outline_id={prev_id} 第{prev_part}部({prev_start}-{prev_end}) 与 part_outline_id={id} 第{part}部({start}-{end})"
            ));
        }
    }
    Ok(())
}

fn validate_text_core_chapter_outline_statuses(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    for outline in &package.text_core.chapter_outlines {
        if !chapter_outline::is_valid_status(&outline.status) {
            return Err(format!(
                "项目族导入包章节大纲状态无效: chapter_outline_id={}, status={}",
                outline.id, outline.status
            ));
        }
    }
    Ok(())
}

fn validate_text_core_chapter_outline_references(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    let outline_chapters = package
        .text_core
        .chapter_outlines
        .iter()
        .map(|outline| (outline.id, outline.chapter_number))
        .collect::<HashMap<_, _>>();

    for chapter in &package.text_core.chapters {
        let Some(outline_id) = chapter.outline_id else {
            continue;
        };
        let Some(outline_chapter_number) = outline_chapters.get(&outline_id).copied() else {
            return Err(format!(
                "项目族导入包章节引用了不存在的章节大纲: chapter_id={}, outline_id={outline_id}",
                chapter.id
            ));
        };
        if outline_chapter_number != chapter.chapter_number {
            return Err(format!(
                "项目族导入包章节引用的章节大纲属于第{outline_chapter_number}章: chapter_id={}, chapter_number={}, outline_id={outline_id}",
                chapter.id, chapter.chapter_number
            ));
        }
    }
    Ok(())
}

fn validate_text_core_chapter_outline_part_references(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    let part_ranges = package
        .text_core
        .part_outlines
        .iter()
        .map(|part| {
            (
                part.id,
                (part.part_number, part.start_chapter, part.end_chapter),
            )
        })
        .collect::<HashMap<_, _>>();

    for outline in &package.text_core.chapter_outlines {
        let Some(part_id) = outline.part_id else {
            continue;
        };
        let Some((part_number, start_chapter, end_chapter)) = part_ranges.get(&part_id).copied()
        else {
            return Err(format!(
                "项目族导入包章节大纲引用了不存在的分部: chapter_outline_id={}, part_id={part_id}",
                outline.id
            ));
        };
        if outline.chapter_number < start_chapter || outline.chapter_number > end_chapter {
            return Err(format!(
                "项目族导入包章节大纲引用的第{part_number}部分部范围不包含该章节: chapter_outline_id={}, chapter_number={}, part_id={part_id}, range={start_chapter}-{end_chapter}",
                outline.id, outline.chapter_number
            ));
        }
    }
    Ok(())
}

fn validate_text_anchor_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    let chapters = package
        .text_core
        .chapters
        .iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<HashMap<_, _>>();
    let source_snapshots = package
        .source_snapshots
        .iter()
        .map(|snapshot| snapshot.id)
        .collect::<HashSet<_>>();
    let snapshot_chapters = package
        .source_snapshot_chapters
        .iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<HashMap<_, _>>();
    let part_outline_ids = package
        .text_core
        .part_outlines
        .iter()
        .map(|outline| outline.id)
        .collect::<HashSet<_>>();
    let chapter_outline_ids = package
        .text_core
        .chapter_outlines
        .iter()
        .map(|outline| outline.id)
        .collect::<HashSet<_>>();
    let blueprint_ids = package
        .text_core
        .blueprint
        .as_ref()
        .map(|blueprint| blueprint.id)
        .into_iter()
        .collect::<HashSet<_>>();
    let blueprint_version_ids = package
        .text_core
        .blueprint_versions
        .iter()
        .map(|version| version.id)
        .collect::<HashSet<_>>();

    for anchor in &package.text_anchors {
        if anchor.project_id != package.root_project_id {
            return Err(format!(
                "项目族导入包文本锚点不属于根项目: text_anchor_id={}, project_id={}",
                anchor.id, anchor.project_id
            ));
        }
        validate_text_anchor_offsets(anchor)?;
        let expected_quote_hash = hash_text(&anchor.quote);
        if anchor.quote_hash != expected_quote_hash {
            return Err(format!(
                "项目族导入包文本锚点 quote_hash 与 quote 不一致: text_anchor_id={}",
                anchor.id
            ));
        }
        match anchor.source_kind.as_str() {
            SOURCE_KIND_CHAPTER_CONTENT => validate_chapter_content_anchor(
                anchor,
                &chapters,
                &source_snapshots,
                &snapshot_chapters,
            )?,
            "chapter_summary" => validate_chapter_anchor_source(
                anchor,
                &chapters,
                &source_snapshots,
                &snapshot_chapters,
            )?,
            "outline" => validate_ambiguous_anchor_source(
                anchor.id,
                anchor.source_id,
                &part_outline_ids,
                &chapter_outline_ids,
                "outline",
            )?,
            "blueprint" => validate_ambiguous_anchor_source(
                anchor.id,
                anchor.source_id,
                &blueprint_ids,
                &blueprint_version_ids,
                "blueprint",
            )?,
            other => {
                return Err(format!(
                    "项目族导入包文本锚点 source_kind 无效: text_anchor_id={}, source_kind={other}",
                    anchor.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_text_anchor_offsets(anchor: &text_anchor::Model) -> Result<(), String> {
    if anchor.paragraph_index < 0
        || anchor.start_offset < 0
        || anchor.end_offset <= anchor.start_offset
    {
        return Err(format!(
            "项目族导入包文本锚点范围无效: text_anchor_id={}, paragraph_index={}, start_offset={}, end_offset={}",
            anchor.id, anchor.paragraph_index, anchor.start_offset, anchor.end_offset
        ));
    }
    Ok(())
}

fn validate_chapter_content_anchor(
    anchor: &text_anchor::Model,
    chapters: &HashMap<i64, &chapter::Model>,
    source_snapshots: &HashSet<i64>,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
) -> Result<(), String> {
    validate_chapter_anchor_source(anchor, chapters, source_snapshots, snapshot_chapters)?;
    let expected_source_version_hash = if anchor.source_snapshot_id.is_some() {
        let snapshot_chapter = snapshot_chapters
            .get(&anchor.source_id)
            .expect("snapshot chapter source was validated before hash check");
        snapshot_chapter.content_hash.as_str()
    } else {
        let chapter = chapters
            .get(&anchor.source_id)
            .expect("chapter source was validated before hash check");
        return validate_text_anchor_source_version_hash(anchor, &hash_text(&chapter.content));
    };
    validate_text_anchor_source_version_hash(anchor, expected_source_version_hash)
}

fn validate_chapter_anchor_source(
    anchor: &text_anchor::Model,
    chapters: &HashMap<i64, &chapter::Model>,
    source_snapshots: &HashSet<i64>,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
) -> Result<(), String> {
    if let Some(snapshot_id) = anchor.source_snapshot_id {
        if !source_snapshots.contains(&snapshot_id) {
            return Err(format!(
                "项目族导入包文本锚点引用了不存在的源文本快照: text_anchor_id={}, source_snapshot_id={snapshot_id}",
                anchor.id
            ));
        }
        let source_snapshot_chapter_id = anchor.source_snapshot_chapter_id.ok_or_else(|| {
            format!(
                "项目族导入包源快照文本锚点缺少 source_snapshot_chapter_id: text_anchor_id={}",
                anchor.id
            )
        })?;
        if source_snapshot_chapter_id != anchor.source_id {
            return Err(format!(
                "项目族导入包源快照文本锚点 source_id 与 source_snapshot_chapter_id 不一致: text_anchor_id={}, source_id={}, source_snapshot_chapter_id={}",
                anchor.id, anchor.source_id, source_snapshot_chapter_id
            ));
        }
        let snapshot_chapter = snapshot_chapters
            .get(&source_snapshot_chapter_id)
            .ok_or_else(|| {
                format!(
                    "项目族导入包文本锚点引用了不存在的源快照章节: text_anchor_id={}, source_snapshot_chapter_id={source_snapshot_chapter_id}",
                    anchor.id
                )
            })?;
        if snapshot_chapter.snapshot_id != snapshot_id {
            return Err(format!(
                "项目族导入包文本锚点源快照章节不属于目标快照: text_anchor_id={}, source_snapshot_id={}, source_snapshot_chapter_id={}",
                anchor.id, snapshot_id, source_snapshot_chapter_id
            ));
        }
        if anchor.source_chapter_id != Some(snapshot_chapter.source_chapter_id) {
            return Err(format!(
                "项目族导入包文本锚点 source_chapter_id 与源快照章节不一致: text_anchor_id={}, source_chapter_id={:?}, expected={}",
                anchor.id, anchor.source_chapter_id, snapshot_chapter.source_chapter_id
            ));
        }
        return Ok(());
    }

    if anchor.source_snapshot_chapter_id.is_some() {
        return Err(format!(
            "项目族导入包非快照文本锚点不能包含 source_snapshot_chapter_id: text_anchor_id={}",
            anchor.id
        ));
    }
    if anchor.source_chapter_id != Some(anchor.source_id) {
        return Err(format!(
            "项目族导入包文本锚点 source_id 与 source_chapter_id 不一致: text_anchor_id={}, source_id={}, source_chapter_id={:?}",
            anchor.id, anchor.source_id, anchor.source_chapter_id
        ));
    }
    if !chapters.contains_key(&anchor.source_id) {
        return Err(format!(
            "项目族导入包文本锚点引用了不存在的章节: text_anchor_id={}, source_id={}",
            anchor.id, anchor.source_id
        ));
    }
    Ok(())
}

fn validate_text_anchor_source_version_hash(
    anchor: &text_anchor::Model,
    expected: &str,
) -> Result<(), String> {
    if anchor.source_version_hash != expected {
        return Err(format!(
            "项目族导入包文本锚点 source_version_hash 与源内容不一致: text_anchor_id={}",
            anchor.id
        ));
    }
    Ok(())
}

fn validate_ambiguous_anchor_source(
    anchor_id: i64,
    source_id: i64,
    first_source_ids: &HashSet<i64>,
    second_source_ids: &HashSet<i64>,
    label: &str,
) -> Result<(), String> {
    match (
        first_source_ids.contains(&source_id),
        second_source_ids.contains(&source_id),
    ) {
        (true, false) | (false, true) => Ok(()),
        (true, true) => Err(format!(
            "项目族导入包文本锚点 {label} source_id 存在歧义: text_anchor_id={}, source_id={source_id}",
            anchor_id
        )),
        (false, false) => Err(format!(
            "项目族导入包文本锚点 {label} source_id 不存在: text_anchor_id={}, source_id={source_id}",
            anchor_id
        )),
    }
}

fn validate_profile_evidence_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    let text_anchor_ids = package
        .text_anchors
        .iter()
        .map(|anchor| anchor.id)
        .collect::<HashSet<_>>();
    let character_profile_ids = package
        .text_core
        .character_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    let object_profile_ids = package
        .text_core
        .object_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    let scene_profile_ids = package
        .text_core
        .scene_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    for evidence in &package.profile_evidence {
        if !text_anchor_ids.contains(&evidence.text_anchor_id) {
            return Err(format!(
                "项目族导入包档案证据引用了不存在的文本锚点: profile_evidence_id={}, text_anchor_id={}",
                evidence.id, evidence.text_anchor_id
            ));
        }
        validate_profile_evidence_profile_id(
            evidence,
            &character_profile_ids,
            &object_profile_ids,
            &scene_profile_ids,
        )?;
        let evidence_role = normalize_profile_evidence_role(&evidence.evidence_role)?;
        if !seen.insert((
            evidence.profile_kind.clone(),
            evidence.profile_id,
            evidence.text_anchor_id,
            evidence_role.to_string(),
        )) {
            return Err(format!(
                "项目族导入包档案证据重复: profile_kind={}, profile_id={}, text_anchor_id={}, evidence_role={}",
                evidence.profile_kind, evidence.profile_id, evidence.text_anchor_id, evidence_role
            ));
        }
    }
    Ok(())
}

fn validate_profile_evidence_profile_id(
    evidence: &profile_evidence::Model,
    character_profile_ids: &HashSet<i64>,
    object_profile_ids: &HashSet<i64>,
    scene_profile_ids: &HashSet<i64>,
) -> Result<(), String> {
    let exists = match evidence.profile_kind.as_str() {
        "character" => character_profile_ids.contains(&evidence.profile_id),
        "object" => object_profile_ids.contains(&evidence.profile_id),
        "scene" => scene_profile_ids.contains(&evidence.profile_id),
        other => {
            return Err(format!(
                "项目族导入包档案证据 profile_kind 无效: profile_evidence_id={}, profile_kind={other}",
                evidence.id
            ));
        }
    };
    if !exists {
        return Err(format!(
            "项目族导入包档案证据引用了不存在的档案: profile_evidence_id={}, profile_kind={}, profile_id={}",
            evidence.id, evidence.profile_kind, evidence.profile_id
        ));
    }
    Ok(())
}

pub(super) fn normalize_profile_evidence_role(role: &str) -> Result<&'static str, String> {
    match role.trim() {
        EVIDENCE_ROLE_FIRST_SEEN => Ok(EVIDENCE_ROLE_FIRST_SEEN),
        EVIDENCE_ROLE_APPEARANCE => Ok(EVIDENCE_ROLE_APPEARANCE),
        EVIDENCE_ROLE_STATE_CHANGE => Ok(EVIDENCE_ROLE_STATE_CHANGE),
        EVIDENCE_ROLE_LOCATION => Ok(EVIDENCE_ROLE_LOCATION),
        EVIDENCE_ROLE_RELATIONSHIP => Ok(EVIDENCE_ROLE_RELATIONSHIP),
        EVIDENCE_ROLE_BEHAVIOR => Ok(EVIDENCE_ROLE_BEHAVIOR),
        "" => Err("项目族导入包档案证据 evidence_role 不能为空".to_string()),
        other => Err(format!("项目族导入包档案证据 evidence_role 无效: {other}")),
    }
}

fn validate_galgame_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    let projects = package
        .projects
        .iter()
        .map(|project| (project.id, project))
        .collect::<HashMap<_, _>>();
    let snapshots = package
        .source_snapshots
        .iter()
        .map(|snapshot| (snapshot.id, snapshot))
        .collect::<HashMap<_, _>>();
    let snapshot_chapters = package
        .source_snapshot_chapters
        .iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<HashMap<_, _>>();
    let character_profile_ids = package
        .text_core
        .character_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    let text_anchors = package
        .text_anchors
        .iter()
        .map(|anchor| (anchor.id, anchor))
        .collect::<HashMap<_, _>>();
    let scripts = package
        .galgame_scripts
        .iter()
        .map(|script| (script.id, script))
        .collect::<HashMap<_, _>>();
    let scenes = package
        .galgame_scenes
        .iter()
        .map(|scene| (scene.id, scene))
        .collect::<HashMap<_, _>>();
    let lines = package
        .galgame_lines
        .iter()
        .map(|line| (line.id, line))
        .collect::<HashMap<_, _>>();

    for script in &package.galgame_scripts {
        let project = projects.get(&script.project_id).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 剧本引用了不存在的项目: script_id={}, project_id={}",
                script.id, script.project_id
            )
        })?;
        if project.project_kind != "galgame" {
            return Err(format!(
                "项目族导入包 Galgame 剧本项目类型无效: script_id={}, project_kind={}",
                script.id, project.project_kind
            ));
        }
        if script.root_project_id != package.root_project_id {
            return Err(format!(
                "项目族导入包 Galgame 剧本 root_project_id 无效: script_id={}",
                script.id
            ));
        }
        if project.source_snapshot_id != Some(script.source_snapshot_id) {
            return Err(format!(
                "项目族导入包 Galgame 剧本源快照与项目不一致: script_id={}, source_snapshot_id={}",
                script.id, script.source_snapshot_id
            ));
        }
        if !snapshots.contains_key(&script.source_snapshot_id) {
            return Err(format!(
                "项目族导入包 Galgame 剧本引用了不存在的源快照: script_id={}, source_snapshot_id={}",
                script.id, script.source_snapshot_id
            ));
        }
    }

    for scene in &package.galgame_scenes {
        let script = scripts.get(&scene.script_id).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 场引用了不存在的剧本: scene_id={}, script_id={}",
                scene.id, scene.script_id
            )
        })?;
        if scene.scene_index <= 0 {
            return Err(format!(
                "项目族导入包 Galgame 场 scene_index 必须是正整数: scene_id={}, scene_index={}",
                scene.id, scene.scene_index
            ));
        }
        if scene.title.trim().is_empty() {
            return Err(format!(
                "项目族导入包 Galgame 场 title 不能为空: scene_id={}",
                scene.id
            ));
        }
        validate_galgame_scene_entry_kind(scene)?;
        let snapshot_chapter = snapshot_chapters
            .get(&scene.source_snapshot_chapter_id)
            .ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 场引用了不存在的源快照章节: scene_id={}, source_snapshot_chapter_id={}",
                    scene.id, scene.source_snapshot_chapter_id
                )
            })?;
        if snapshot_chapter.snapshot_id != script.source_snapshot_id {
            return Err(format!(
                "项目族导入包 Galgame 场源快照章节与剧本源快照不一致: scene_id={}",
                scene.id
            ));
        }
        if let Some(next_scene_id) = scene.next_scene_id
            && !scenes.contains_key(&next_scene_id)
        {
            return Err(format!(
                "项目族导入包 Galgame 场 next_scene_id 不存在: scene_id={}, next_scene_id={next_scene_id}",
                scene.id
            ));
        }
        if scene.next_scene_id == Some(scene.id) {
            return Err(format!(
                "项目族导入包 Galgame 场 next_scene_id 不能指向自身: scene_id={}",
                scene.id
            ));
        }
        if let Some(next_scene_id) = scene.next_scene_id {
            let next_scene = scenes.get(&next_scene_id).ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 场 next_scene_id 不存在: scene_id={}, next_scene_id={next_scene_id}",
                    scene.id
                )
            })?;
            if next_scene.script_id != scene.script_id {
                return Err(format!(
                    "项目族导入包 Galgame 场 next_scene_id 不属于同一剧本: scene_id={}, next_scene_id={next_scene_id}",
                    scene.id
                ));
            }
        }
        if scene.entry_kind == "branch" {
            let Some(next_scene_id) = scene.next_scene_id else {
                return Err(format!(
                    "项目族导入包 Galgame branch 场缺少 next_scene_id: scene_id={}",
                    scene.id
                ));
            };
            let next_scene = scenes.get(&next_scene_id).ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 场 next_scene_id 不存在: scene_id={}, next_scene_id={next_scene_id}",
                    scene.id
                )
            })?;
            if next_scene.entry_kind != "main" {
                return Err(format!(
                    "项目族导入包 Galgame branch 场 next_scene_id 必须指向主线场: scene_id={}, next_scene_id={next_scene_id}",
                    scene.id
                ));
            }
        }
        validate_galgame_participants_json(
            scene.id,
            &scene.participants_json,
            &character_profile_ids,
        )?;
    }

    let mut lines_by_scene = HashMap::<i64, Vec<&galgame_line::Model>>::new();
    for line in &package.galgame_lines {
        let scene = scenes.get(&line.scene_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 行引用了不存在的场: line_id={}, scene_id={}",
                line.id, line.scene_id
            )
        })?;
        validate_galgame_line_semantics(line)?;
        if let Some(profile_id) = line.speaker_profile_id
            && !character_profile_ids.contains(&profile_id)
        {
            return Err(format!(
                "项目族导入包 Galgame 行 speaker_profile_id 不存在: line_id={}, speaker_profile_id={profile_id}",
                line.id
            ));
        }
        if let Some(anchor_id) = line.text_anchor_id {
            let anchor = text_anchors.get(&anchor_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 行 text_anchor_id 不存在: line_id={}, text_anchor_id={anchor_id}",
                    line.id
                )
            })?;
            let script = scripts.get(&scene.script_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 行所属场引用了不存在的剧本: line_id={}, scene_id={}, script_id={}",
                    line.id, scene.id, scene.script_id
                )
            })?;
            if anchor.source_snapshot_id != Some(script.source_snapshot_id)
                || anchor.source_snapshot_chapter_id != Some(scene.source_snapshot_chapter_id)
            {
                return Err(format!(
                    "项目族导入包 Galgame 行 text_anchor_id 不属于行所属场源快照章节: line_id={}, text_anchor_id={anchor_id}, source_snapshot_chapter_id={:?}, expected_source_snapshot_chapter_id={}",
                    line.id, anchor.source_snapshot_chapter_id, scene.source_snapshot_chapter_id
                ));
            }
        }
        lines_by_scene.entry(line.scene_id).or_default().push(line);
    }

    for (scene_id, scene_lines) in &lines_by_scene {
        let mut line_indexes = std::collections::BTreeSet::new();
        for line in scene_lines {
            if !line_indexes.insert(line.line_index) {
                return Err(format!(
                    "项目族导入包 Galgame 行 line_index 重复: scene_id={}, line_id={}, line_index={}",
                    scene_id, line.id, line.line_index
                ));
            }
        }
        let expected_indexes =
            (1..=scene_lines.len() as i32).collect::<std::collections::BTreeSet<_>>();
        if line_indexes != expected_indexes {
            return Err(format!(
                "项目族导入包 Galgame 行 line_index 必须从 1 连续编号: scene_id={}, line_indexes={:?}, expected={:?}",
                scene_id, line_indexes, expected_indexes
            ));
        }
    }

    let mut choices_by_line = HashMap::<i64, Vec<&galgame_choice::Model>>::new();
    for choice in &package.galgame_choices {
        choices_by_line
            .entry(choice.line_id)
            .or_default()
            .push(choice);
    }
    for line in package
        .galgame_lines
        .iter()
        .filter(|line| line.line_type == "choice")
    {
        let choices = choices_by_line
            .get(&line.id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if !(2..=3).contains(&choices.len()) {
            return Err(format!(
                "项目族导入包 Galgame choice 行选项数量必须是 2 到 3 个: line_id={}, choice_count={}",
                line.id,
                choices.len()
            ));
        }
        let mut option_indexes = std::collections::BTreeSet::new();
        for choice in choices {
            if choice.option_text.trim().is_empty() {
                return Err(format!(
                    "项目族导入包 Galgame 选项 option_text 不能为空: choice_id={}, line_id={}",
                    choice.id, choice.line_id
                ));
            }
            if choice.option_index <= 0 {
                return Err(format!(
                    "项目族导入包 Galgame 选项 option_index 必须为正数: choice_id={}, line_id={}, option_index={}",
                    choice.id, choice.line_id, choice.option_index
                ));
            }
            if !option_indexes.insert(choice.option_index) {
                return Err(format!(
                    "项目族导入包 Galgame 选项 option_index 重复: choice_id={}, line_id={}, option_index={}",
                    choice.id, choice.line_id, choice.option_index
                ));
            }
        }
        let expected_indexes =
            (1..=choices.len() as i32).collect::<std::collections::BTreeSet<_>>();
        if option_indexes != expected_indexes {
            return Err(format!(
                "项目族导入包 Galgame 选项 option_index 必须从 1 连续编号: line_id={}, option_indexes={:?}, expected={:?}",
                line.id, option_indexes, expected_indexes
            ));
        }
    }

    for choice in &package.galgame_choices {
        let line = lines.get(&choice.line_id).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 选项引用了不存在的行: choice_id={}, line_id={}",
                choice.id, choice.line_id
            )
        })?;
        if line.line_type != "choice" {
            return Err(format!(
                "项目族导入包 Galgame 选项必须挂在 choice 行下: choice_id={}, line_id={}",
                choice.id, choice.line_id
            ));
        }
        let line_scene = scenes.get(&line.scene_id).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 选项所属行引用了不存在的场: choice_id={}, line_id={}, scene_id={}",
                choice.id, line.id, line.scene_id
            )
        })?;
        let target_scene_id = choice.target_scene_id.ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 选项缺少 target_scene_id: choice_id={}",
                choice.id
            )
        })?;
        let target_scene = scenes.get(&target_scene_id).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 选项 target_scene_id 不存在: choice_id={}, target_scene_id={target_scene_id}",
                choice.id
            )
        })?;
        if target_scene.script_id != line_scene.script_id {
            return Err(format!(
                "项目族导入包 Galgame 选项 target_scene_id 不属于同一剧本: choice_id={}, line_id={}, target_scene_id={target_scene_id}",
                choice.id, line.id
            ));
        }
        if target_scene.entry_kind == "branch" {
            let return_scene_id = target_scene.next_scene_id.ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 选项 target_scene_id 指向未回收敛支线: choice_id={}, target_scene_id={target_scene_id}",
                    choice.id
                )
            })?;
            let return_scene = scenes.get(&return_scene_id).ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 选项 target_scene_id 回收敛目标不存在: choice_id={}, target_scene_id={target_scene_id}, next_scene_id={return_scene_id}",
                    choice.id
                )
            })?;
            if return_scene.script_id != line_scene.script_id {
                return Err(format!(
                    "项目族导入包 Galgame 选项 target_scene_id 回收敛目标不属于同一剧本: choice_id={}, target_scene_id={target_scene_id}, next_scene_id={return_scene_id}",
                    choice.id
                ));
            }
            if return_scene.entry_kind != "main" {
                return Err(format!(
                    "项目族导入包 Galgame 选项 target_scene_id 回收敛目标必须是主线场: choice_id={}, target_scene_id={target_scene_id}, next_scene_id={return_scene_id}",
                    choice.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_galgame_scene_entry_kind(scene: &galgame_scene::Model) -> Result<(), String> {
    match scene.entry_kind.as_str() {
        "main" | "branch" => Ok(()),
        other => Err(format!(
            "项目族导入包 Galgame 场 entry_kind 无效: scene_id={}, entry_kind={other}",
            scene.id
        )),
    }
}

fn validate_galgame_line_semantics(line: &galgame_line::Model) -> Result<(), String> {
    if line.line_index <= 0 {
        return Err(format!(
            "项目族导入包 Galgame 行 line_index 必须是正整数: line_id={}, line_index={}",
            line.id, line.line_index
        ));
    }
    if line.text.trim().is_empty() {
        return Err(format!(
            "项目族导入包 Galgame 行 text 不能为空: line_id={}",
            line.id
        ));
    }
    match line.line_type.as_str() {
        "dialogue" | "monologue" if line.speaker_profile_id.is_none() => Err(format!(
            "项目族导入包 Galgame {} 行必须绑定 speaker_profile_id: line_id={}",
            line.line_type, line.id
        )),
        "dialogue" | "monologue" if normalized_galgame_expression(line).is_none() => Err(format!(
            "项目族导入包 Galgame {} 行必须显式提供 expression: line_id={}",
            line.line_type, line.id
        )),
        "dialogue" | "monologue" => Ok(()),
        "narration" | "stage_direction" | "choice" if line.speaker_profile_id.is_some() => {
            Err(format!(
                "项目族导入包 Galgame {} 行不能绑定 speaker_profile_id: line_id={}",
                line.line_type, line.id
            ))
        }
        "narration" | "stage_direction" | "choice" => {
            normalized_galgame_expression(line);
            Ok(())
        }
        other => Err(format!(
            "项目族导入包 Galgame 行类型无效: line_id={}, line_type={other}",
            line.id
        )),
    }?;
    validate_galgame_line_presentation_json(line)?;
    if let Some(expression) = normalized_galgame_expression(line)
        && !is_valid_galgame_expression(expression)
    {
        return Err(format!(
            "项目族导入包 Galgame 行 expression 无效: line_id={}, expression={expression}",
            line.id
        ));
    }
    if let Some(position) = normalized_galgame_position(line)
        && !is_valid_galgame_position(position)
    {
        return Err(format!(
            "项目族导入包 Galgame 行 position 无效: line_id={}, position={position}",
            line.id
        ));
    }
    Ok(())
}

fn validate_galgame_line_presentation_json(line: &galgame_line::Model) -> Result<(), String> {
    let object = line.presentation_json.as_object().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame 行 presentation_json 必须是 JSON 对象: line_id={}",
            line.id
        )
    })?;
    let cg_enabled = match object.get("cg") {
        Some(Value::Bool(value)) => *value,
        Some(Value::Null) | None => false,
        Some(_) => {
            return Err(format!(
                "项目族导入包 Galgame 行 presentation_json.cg 必须是布尔值: line_id={}",
                line.id
            ));
        }
    };
    if let Some(value) = object.get("cg_importance")
        && !value.is_null()
    {
        let importance = value.as_str().map(str::trim).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 行 presentation_json.cg_importance 必须是字符串: line_id={}",
                line.id
            )
        })?;
        if !importance.is_empty() {
            ensure_package!(
                GALGAME_CG_IMPORTANCE.contains(&importance),
                "项目族导入包 Galgame 行 presentation_json.cg_importance 无效: line_id={}, cg_importance={}",
                line.id,
                importance
            );
        }
    }
    let note = galgame_event_cg_note_text(object);
    if cg_enabled || note.is_some() {
        ensure_package!(
            note.is_some(),
            "项目族导入包 Galgame 行启用事件 CG 时必须提供 cg_note: line_id={}",
            line.id
        );
    }
    Ok(())
}

fn galgame_event_cg_note_text(object: &serde_json::Map<String, Value>) -> Option<&str> {
    for key in ["cg_note", "cg_prompt", "event_cg_note"] {
        if let Some(value) = object
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(value);
        }
    }
    None
}

fn normalized_galgame_expression(line: &galgame_line::Model) -> Option<&str> {
    line.expression
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn normalized_galgame_position(line: &galgame_line::Model) -> Option<&str> {
    line.position
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn is_valid_galgame_expression(value: &str) -> bool {
    matches!(
        value,
        "neutral" | "smile" | "sad" | "angry" | "surprised" | "shy" | "serious"
    )
}

fn is_valid_galgame_position(value: &str) -> bool {
    matches!(value, "left" | "center" | "right")
}

fn validate_galgame_participants_json(
    scene_id: i64,
    value: &Value,
    character_profile_ids: &HashSet<i64>,
) -> Result<(), String> {
    let Some(items) = value.as_array() else {
        return Err(format!(
            "项目族导入包 Galgame 场 participants_json 必须是数组: scene_id={scene_id}"
        ));
    };
    for item in items {
        let object = item.as_object().ok_or_else(|| {
            format!("项目族导入包 Galgame 场 participants_json 项必须是对象: scene_id={scene_id}")
        })?;
        let Some(profile_id_value) = object.get("character_profile_id") else {
            continue;
        };
        let Some(profile_id) = profile_id_value.as_i64() else {
            return Err(format!(
                "项目族导入包 Galgame 场 participants_json.character_profile_id 必须是正整数: scene_id={scene_id}"
            ));
        };
        if profile_id <= 0 {
            return Err(format!(
                "项目族导入包 Galgame 场 participants_json.character_profile_id 必须是正整数: scene_id={}, character_profile_id={profile_id}",
                scene_id
            ));
        }
        if !character_profile_ids.contains(&profile_id) {
            return Err(format!(
                "项目族导入包 Galgame 场 participants_json.character_profile_id 不存在: scene_id={}, character_profile_id={profile_id}",
                scene_id
            ));
        }
    }
    Ok(())
}

fn validate_generation_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    for spec in &package.generation_specs {
        ensure_package!(
            is_valid_generation_spec_target_kind(&spec.target_kind),
            "项目族导入包生成意图目标类型无效: generation_spec_id={}, target_kind={}",
            spec.id,
            spec.target_kind
        );
        ensure_package!(
            is_valid_generation_spec_target_entity_kind(&spec.target_entity_kind),
            "项目族导入包生成意图目标实体类型无效: generation_spec_id={}, target_entity_kind={}",
            spec.id,
            spec.target_entity_kind
        );
        ensure_package!(
            is_valid_generation_spec_status(&spec.status),
            "项目族导入包生成意图状态无效: generation_spec_id={}, status={}",
            spec.id,
            spec.status
        );
        ensure_package!(
            spec.params_json.is_object(),
            "项目族导入包生成意图参数必须是 JSON 对象: generation_spec_id={}",
            spec.id
        );
    }

    for record in &package.generation_spec_text_anchors {
        ensure_package!(
            is_valid_generation_text_anchor_role(&record.role),
            "项目族导入包生成意图文本锚点角色无效: generation_spec_id={}, text_anchor_id={}, role={}",
            record.generation_spec_id,
            record.text_anchor_id,
            record.role
        );
    }

    for record in &package.generation_spec_entity_refs {
        ensure_package!(
            is_valid_generation_entity_kind(&record.entity_kind),
            "项目族导入包生成意图实体类型无效: generation_spec_id={}, entity_id={}, entity_kind={}",
            record.generation_spec_id,
            record.entity_id,
            record.entity_kind
        );
        ensure_package!(
            is_valid_generation_entity_role(&record.role),
            "项目族导入包生成意图实体角色无效: generation_spec_id={}, entity_id={}, role={}",
            record.generation_spec_id,
            record.entity_id,
            record.role
        );
    }

    for record in &package.generation_spec_media_refs {
        ensure_package!(
            is_valid_generation_media_ref_role(&record.role),
            "项目族导入包生成意图媒体角色无效: generation_spec_id={}, media_asset_id={}, role={}",
            record.generation_spec_id,
            record.media_asset_id,
            record.role
        );
    }

    for artifact in &package.prompt_artifacts {
        ensure_package!(
            is_valid_generation_prompt_kind(&artifact.prompt_kind),
            "项目族导入包提示词类型无效: prompt_artifact_id={}, prompt_kind={}",
            artifact.id,
            artifact.prompt_kind
        );
    }

    validate_generation_reference_manifest(package)?;
    Ok(())
}

fn validate_generation_reference_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    let specs = package
        .generation_specs
        .iter()
        .map(|spec| (spec.id, spec))
        .collect::<HashMap<_, _>>();
    let source_snapshots = package
        .source_snapshots
        .iter()
        .map(|snapshot| (snapshot.id, snapshot))
        .collect::<HashMap<_, _>>();
    let snapshot_chapters = package
        .source_snapshot_chapters
        .iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<HashMap<_, _>>();
    let snapshot_profiles = package
        .source_snapshot_profiles
        .iter()
        .map(|profile| {
            (
                (
                    profile.snapshot_id,
                    snapshot_profile_key(&profile.profile_kind, profile.source_profile_id),
                ),
                profile,
            )
        })
        .collect::<HashMap<_, _>>();
    let text_anchors = package
        .text_anchors
        .iter()
        .map(|anchor| (anchor.id, anchor))
        .collect::<HashMap<_, _>>();
    let media_assets = package
        .media_assets
        .iter()
        .map(|asset| (asset.id, asset))
        .collect::<HashMap<_, _>>();
    let video_segment_ids = package
        .video_segments
        .iter()
        .map(|segment| segment.id)
        .collect::<HashSet<_>>();
    let assembly_timeline_ids = package
        .assembly_timelines
        .iter()
        .map(|timeline| timeline.id)
        .collect::<HashSet<_>>();
    let galgame_scripts = package
        .galgame_scripts
        .iter()
        .map(|script| (script.id, script))
        .collect::<HashMap<_, _>>();
    let galgame_scenes = package
        .galgame_scenes
        .iter()
        .map(|scene| (scene.id, scene))
        .collect::<HashMap<_, _>>();
    let galgame_lines = package
        .galgame_lines
        .iter()
        .map(|line| (line.id, line))
        .collect::<HashMap<_, _>>();

    for spec in &package.generation_specs {
        let source_snapshot_id = required_generation_spec_source_snapshot_id(spec)?;
        let snapshot = source_snapshots
            .get(&source_snapshot_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包生成意图引用了不存在的源文本快照: generation_spec_id={}, source_snapshot_id={}",
                    spec.id, source_snapshot_id
                )
            })?;
        if spec.source_version_hash != snapshot.source_version_hash {
            return Err(format!(
                "项目族导入包生成意图 source_version_hash 与源快照不一致: generation_spec_id={}, source_snapshot_id={}",
                spec.id, source_snapshot_id
            ));
        }
        validate_generation_spec_source_chapter(spec, source_snapshot_id, &snapshot_chapters)?;
        validate_generation_spec_target_entity(
            spec,
            source_snapshot_id,
            &snapshot_chapters,
            &snapshot_profiles,
            &video_segment_ids,
            &assembly_timeline_ids,
            &galgame_scripts,
            &galgame_scenes,
            &galgame_lines,
        )?;
        validate_galgame_generation_spec_params(
            spec,
            source_snapshot_id,
            &snapshot_chapters,
            &snapshot_profiles,
            &galgame_scripts,
            &galgame_scenes,
            &galgame_lines,
        )?;
    }
    for record in &package.generation_spec_text_anchors {
        let spec = specs.get(&record.generation_spec_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包生成意图文本锚点引用了不存在的生成意图: generation_spec_id={}, text_anchor_id={}",
                record.generation_spec_id, record.text_anchor_id
            )
        })?;
        let anchor = text_anchors.get(&record.text_anchor_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包生成意图文本锚点引用了不存在的文本锚点: generation_spec_id={}, text_anchor_id={}",
                record.generation_spec_id, record.text_anchor_id
            )
        })?;
        let source_snapshot_id = required_generation_spec_source_snapshot_id(spec)?;
        if anchor.source_snapshot_id != Some(source_snapshot_id) {
            return Err(format!(
                "项目族导入包生成意图文本锚点不属于源快照: generation_spec_id={}, text_anchor_id={}, source_snapshot_id={:?}, expected={}",
                spec.id, anchor.id, anchor.source_snapshot_id, source_snapshot_id
            ));
        }
        if let Some(source_snapshot_chapter_id) = spec.source_snapshot_chapter_id
            && anchor.source_snapshot_chapter_id != Some(source_snapshot_chapter_id)
        {
            return Err(format!(
                "项目族导入包生成意图文本锚点不属于生成意图源快照章节: generation_spec_id={}, text_anchor_id={}, source_snapshot_chapter_id={:?}, expected={}",
                spec.id, anchor.id, anchor.source_snapshot_chapter_id, source_snapshot_chapter_id
            ));
        }
    }
    for record in &package.generation_spec_entity_refs {
        let spec = specs.get(&record.generation_spec_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包生成意图实体引用了不存在的生成意图: generation_spec_id={}, entity_kind={}, entity_id={}",
                record.generation_spec_id, record.entity_kind, record.entity_id
            )
        })?;
        let source_snapshot_id = required_generation_spec_source_snapshot_id(spec)?;
        if !snapshot_profiles.contains_key(&(
            source_snapshot_id,
            snapshot_profile_key(&record.entity_kind, record.entity_id),
        )) {
            return Err(format!(
                "项目族导入包生成意图实体引用不属于源快照: generation_spec_id={}, entity_kind={}, entity_id={}, source_snapshot_id={}",
                spec.id, record.entity_kind, record.entity_id, source_snapshot_id
            ));
        }
    }
    for record in &package.generation_spec_media_refs {
        let spec = specs.get(&record.generation_spec_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包生成意图媒体引用了不存在的生成意图: generation_spec_id={}, media_asset_id={}",
                record.generation_spec_id, record.media_asset_id
            )
        })?;
        let asset = media_assets.get(&record.media_asset_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包生成意图媒体引用了不存在的媒体资产: generation_spec_id={}, media_asset_id={}",
                record.generation_spec_id, record.media_asset_id
            )
        })?;
        if asset.project_id != spec.project_id || asset.root_project_id != spec.root_project_id {
            return Err(format!(
                "项目族导入包生成意图媒体引用跨项目: generation_spec_id={}, media_asset_id={}",
                spec.id, asset.id
            ));
        }
        if asset.status != MEDIA_STATUS_COMPLETED || asset.file_path.is_none() {
            return Err(format!(
                "项目族导入包生成意图媒体引用不是可用已完成资产: generation_spec_id={}, media_asset_id={}",
                spec.id, asset.id
            ));
        }
    }
    for artifact in &package.prompt_artifacts {
        if !specs.contains_key(&artifact.generation_spec_id) {
            return Err(format!(
                "项目族导入包提示词引用了不存在的生成意图: prompt_artifact_id={}, generation_spec_id={}",
                artifact.id, artifact.generation_spec_id
            ));
        }
    }
    Ok(())
}

fn required_generation_spec_source_snapshot_id(
    spec: &generation_spec::Model,
) -> Result<i64, String> {
    let context_kind = spec
        .source_context_kind
        .as_deref()
        .unwrap_or("snapshot_backed");
    if context_kind != "snapshot_backed" {
        return Err(format!(
            "项目族导入包生成意图只支持 snapshot_backed 上下文: generation_spec_id={}, source_context_kind={context_kind}",
            spec.id
        ));
    }
    spec.source_snapshot_id.ok_or_else(|| {
        format!(
            "项目族导入包生成意图缺少 source_snapshot_id: generation_spec_id={}",
            spec.id
        )
    })
}

fn validate_generation_spec_source_chapter(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
) -> Result<(), String> {
    if spec.source_chapter_id.is_some() != spec.source_snapshot_chapter_id.is_some() {
        return Err(format!(
            "项目族导入包生成意图 source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空: generation_spec_id={}",
            spec.id
        ));
    }
    let Some(snapshot_chapter_id) = spec.source_snapshot_chapter_id else {
        return Ok(());
    };
    let snapshot_chapter = snapshot_chapters.get(&snapshot_chapter_id).ok_or_else(|| {
        format!(
            "项目族导入包生成意图引用了不存在的源快照章节: generation_spec_id={}, source_snapshot_chapter_id={snapshot_chapter_id}",
            spec.id
        )
    })?;
    if snapshot_chapter.snapshot_id != source_snapshot_id {
        return Err(format!(
            "项目族导入包生成意图源快照章节不属于目标快照: generation_spec_id={}, source_snapshot_id={}, source_snapshot_chapter_id={snapshot_chapter_id}",
            spec.id, source_snapshot_id
        ));
    }
    if spec.source_chapter_id != Some(snapshot_chapter.source_chapter_id) {
        return Err(format!(
            "项目族导入包生成意图 source_chapter_id 与源快照章节不一致: generation_spec_id={}, source_chapter_id={:?}, expected={}",
            spec.id, spec.source_chapter_id, snapshot_chapter.source_chapter_id
        ));
    }
    Ok(())
}

fn validate_generation_spec_target_entity(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
    video_segment_ids: &HashSet<i64>,
    assembly_timeline_ids: &HashSet<i64>,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
    galgame_lines: &HashMap<i64, &galgame_line::Model>,
) -> Result<(), String> {
    let Some(target_entity_id) = spec.target_entity_id else {
        return Ok(());
    };
    match spec.target_entity_kind.as_str() {
        "character" | "character_profile" | "object" | "scene"
            if !snapshot_profiles.contains_key(&(
                source_snapshot_id,
                snapshot_profile_key(&spec.target_entity_kind, target_entity_id),
            )) =>
        {
            return Err(format!(
                "项目族导入包生成意图目标实体不属于源快照: generation_spec_id={}, target_entity_kind={}, target_entity_id={}, source_snapshot_id={}",
                spec.id, spec.target_entity_kind, target_entity_id, source_snapshot_id
            ));
        }
        "chapter" => {
            let snapshot_chapter = snapshot_chapters.get(&target_entity_id).ok_or_else(|| {
                format!(
                    "项目族导入包生成意图章节目标不存在: generation_spec_id={}, target_entity_id={target_entity_id}",
                    spec.id
                )
            })?;
            if snapshot_chapter.snapshot_id != source_snapshot_id {
                return Err(format!(
                    "项目族导入包生成意图章节目标不属于源快照: generation_spec_id={}, target_entity_id={}, source_snapshot_id={}",
                    spec.id, target_entity_id, source_snapshot_id
                ));
            }
        }
        "segment" if !video_segment_ids.contains(&target_entity_id) => {
            return Err(format!(
                "项目族导入包生成意图视频片段目标不存在: generation_spec_id={}, target_entity_id={target_entity_id}",
                spec.id
            ));
        }
        "assembly" if !assembly_timeline_ids.contains(&target_entity_id) => {
            return Err(format!(
                "项目族导入包生成意图成片时间线目标不存在: generation_spec_id={}, target_entity_id={target_entity_id}",
                spec.id
            ));
        }
        "galgame_scene" => {
            let scene = galgame_scenes.get(&target_entity_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包生成意图 Galgame 场目标不存在: generation_spec_id={}, target_entity_id={target_entity_id}",
                    spec.id
                )
            })?;
            validate_generation_spec_galgame_scene_context(
                spec,
                source_snapshot_id,
                scene,
                galgame_scripts,
            )?;
        }
        "galgame_line" => {
            let line = galgame_lines.get(&target_entity_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包生成意图 Galgame 行目标不存在: generation_spec_id={}, target_entity_id={target_entity_id}",
                    spec.id
                )
            })?;
            let scene = galgame_scenes.get(&line.scene_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包生成意图 Galgame 行所属场不存在: generation_spec_id={}, target_entity_id={}, scene_id={}",
                    spec.id, target_entity_id, line.scene_id
                )
            })?;
            validate_generation_spec_galgame_scene_context(
                spec,
                source_snapshot_id,
                scene,
                galgame_scripts,
            )?;
        }
        "segment" | "assembly" => {}
        _ => {}
    }
    Ok(())
}

fn validate_generation_spec_galgame_scene_context(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    scene: &galgame_scene::Model,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
) -> Result<(), String> {
    let script = galgame_scripts.get(&scene.script_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包生成意图 Galgame 目标场引用了不存在的剧本: generation_spec_id={}, scene_id={}, script_id={}",
            spec.id, scene.id, scene.script_id
        )
    })?;
    if script.project_id != spec.project_id
        || script.root_project_id != spec.root_project_id
        || script.source_snapshot_id != source_snapshot_id
    {
        return Err(format!(
            "项目族导入包生成意图 Galgame 目标上下文不一致: generation_spec_id={}, target_project_id={}, target_root_project_id={}, target_source_snapshot_id={}, spec_project_id={}, spec_root_project_id={}, spec_source_snapshot_id={}",
            spec.id,
            script.project_id,
            script.root_project_id,
            script.source_snapshot_id,
            spec.project_id,
            spec.root_project_id,
            source_snapshot_id
        ));
    }
    Ok(())
}

fn validate_galgame_generation_spec_params(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
    galgame_lines: &HashMap<i64, &galgame_line::Model>,
) -> Result<(), String> {
    match (spec.target_kind.as_str(), spec.target_entity_kind.as_str()) {
        ("galgame_background", "galgame_scene") => validate_galgame_background_spec_params(
            spec,
            source_snapshot_id,
            snapshot_chapters,
            galgame_scripts,
            galgame_scenes,
        ),
        ("galgame_cg", "galgame_line") => validate_galgame_cg_spec_params(
            spec,
            source_snapshot_id,
            snapshot_profiles,
            galgame_scripts,
            galgame_scenes,
            galgame_lines,
        ),
        ("portrait_expression", "character_profile") => validate_galgame_portrait_spec_params(
            spec,
            source_snapshot_id,
            snapshot_profiles,
            galgame_scripts,
            galgame_scenes,
            galgame_lines,
        ),
        _ => Ok(()),
    }
}

fn validate_galgame_background_spec_params(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
) -> Result<(), String> {
    let target_id = required_generation_spec_target_id(spec)?;
    let target_scene = galgame_scenes.get(&target_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame 背景参数目标场不存在: generation_spec_id={}, target_entity_id={target_id}",
            spec.id
        )
    })?;
    validate_generation_spec_galgame_scene_context(
        spec,
        source_snapshot_id,
        target_scene,
        galgame_scripts,
    )?;
    let params = generation_spec_params_object(spec)?;
    let expected_key = format!("galgame:{}:background:scene:{target_id}", spec.project_id);
    validate_generation_spec_canonical_key(spec, params, &expected_key)?;
    let scene = required_params_object(params, "scene", spec.id)?;
    validate_params_id_equals(spec, scene, "scene.id", target_id)?;
    let source = required_params_object(params, "source", spec.id)?;
    let source_snapshot_chapter_id =
        required_params_i64(source, "source.source_snapshot_chapter_id", spec.id)?;
    ensure_package!(
        spec.source_snapshot_chapter_id == Some(source_snapshot_chapter_id),
        "项目族导入包 Galgame 背景参数 source_snapshot_chapter_id 与生成意图不一致: generation_spec_id={}, params_source_snapshot_chapter_id={}, spec_source_snapshot_chapter_id={:?}",
        spec.id,
        source_snapshot_chapter_id,
        spec.source_snapshot_chapter_id
    );
    let snapshot_chapter = snapshot_chapters
        .get(&source_snapshot_chapter_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 背景参数 source_snapshot_chapter_id 不存在: generation_spec_id={}, source_snapshot_chapter_id={source_snapshot_chapter_id}",
                spec.id
            )
        })?;
    ensure_package!(
        snapshot_chapter.snapshot_id == source_snapshot_id,
        "项目族导入包 Galgame 背景参数 source_snapshot_chapter_id 不属于源快照: generation_spec_id={}, source_snapshot_id={}, source_snapshot_chapter_id={}",
        spec.id,
        source_snapshot_id,
        source_snapshot_chapter_id
    );
    let source_chapter_id = required_params_i64(source, "source.source_chapter_id", spec.id)?;
    ensure_package!(
        spec.source_chapter_id == Some(source_chapter_id)
            && snapshot_chapter.source_chapter_id == source_chapter_id,
        "项目族导入包 Galgame 背景参数 source_chapter_id 与源快照章节不一致: generation_spec_id={}, params_source_chapter_id={}, spec_source_chapter_id={:?}, expected={}",
        spec.id,
        source_chapter_id,
        spec.source_chapter_id,
        snapshot_chapter.source_chapter_id
    );
    Ok(())
}

fn validate_galgame_cg_spec_params(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
    galgame_lines: &HashMap<i64, &galgame_line::Model>,
) -> Result<(), String> {
    let target_id = required_generation_spec_target_id(spec)?;
    let target_line = galgame_lines.get(&target_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame CG 参数目标行不存在: generation_spec_id={}, target_entity_id={target_id}",
            spec.id
        )
    })?;
    let params = generation_spec_params_object(spec)?;
    let expected_key = format!("galgame:{}:cg:line:{target_id}", spec.project_id);
    validate_generation_spec_canonical_key(spec, params, &expected_key)?;
    let line = required_params_object(params, "line", spec.id)?;
    validate_params_id_equals(spec, line, "line.id", target_id)?;
    if let Some(profile_id) = optional_params_i64(line, "line.speaker_profile_id", spec.id)? {
        validate_snapshot_character_profile_id(
            spec,
            source_snapshot_id,
            profile_id,
            snapshot_profiles,
        )?;
    }
    let scene = required_params_object(params, "scene", spec.id)?;
    let scene_id = required_params_i64(scene, "scene.id", spec.id)?;
    let target_scene = galgame_scenes.get(&scene_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame CG 参数 scene.id 不存在: generation_spec_id={}, scene_id={scene_id}",
            spec.id
        )
    })?;
    ensure_package!(
        target_line.scene_id == scene_id,
        "项目族导入包 Galgame CG 参数 scene.id 与目标行所属场不一致: generation_spec_id={}, line_id={}, params_scene_id={}, actual_scene_id={}",
        spec.id,
        target_id,
        scene_id,
        target_line.scene_id
    );
    validate_generation_spec_galgame_scene_context(
        spec,
        source_snapshot_id,
        target_scene,
        galgame_scripts,
    )?;
    required_params_string(params, "cg_note", spec.id)?;
    validate_optional_galgame_cg_importance(params, spec.id)?;
    Ok(())
}

fn validate_galgame_portrait_spec_params(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
    galgame_lines: &HashMap<i64, &galgame_line::Model>,
) -> Result<(), String> {
    let target_id = required_generation_spec_target_id(spec)?;
    ensure_package!(
        spec.source_chapter_id.is_none() && spec.source_snapshot_chapter_id.is_none(),
        "项目族导入包 Galgame 立绘差分生成意图必须使用源快照级上下文: generation_spec_id={}, source_chapter_id={:?}, source_snapshot_chapter_id={:?}",
        spec.id,
        spec.source_chapter_id,
        spec.source_snapshot_chapter_id
    );
    validate_snapshot_character_profile_id(spec, source_snapshot_id, target_id, snapshot_profiles)?;
    let params = generation_spec_params_object(spec)?;
    let character_profile_id = required_params_i64(params, "character_profile_id", spec.id)?;
    ensure_package!(
        character_profile_id == target_id,
        "项目族导入包 Galgame 立绘差分参数 character_profile_id 与 target_entity_id 不一致: generation_spec_id={}, character_profile_id={}, target_entity_id={}",
        spec.id,
        character_profile_id,
        target_id
    );
    let expression = required_params_string(params, "expression", spec.id)?;
    ensure_package!(
        is_valid_galgame_expression(&expression),
        "项目族导入包 Galgame 立绘差分参数 expression 无效: generation_spec_id={}, expression={}",
        spec.id,
        expression
    );
    let pose = optional_params_string(params, "pose", spec.id)?;
    let variant_key = match pose {
        Some(ref pose) => format!("expression:{expression}:pose:{pose}"),
        None => format!("expression:{expression}"),
    };
    let expected_key = format!(
        "galgame:{}:portrait:{target_id}:{variant_key}",
        spec.project_id
    );
    validate_generation_spec_canonical_key(spec, params, &expected_key)?;
    let source_line_ids = required_params_i64_array(params, "source_line_ids", spec.id)?;
    let position_samples = required_params_string_array(params, "position_samples", spec.id)?;
    let mut source_positions = BTreeSet::new();
    for line_id in source_line_ids {
        let line = galgame_lines.get(&line_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 立绘差分参数 source_line_ids 引用了不存在的行: generation_spec_id={}, line_id={line_id}",
                spec.id
            )
        })?;
        ensure_package!(
            line.speaker_profile_id == Some(target_id),
            "项目族导入包 Galgame 立绘差分参数 source_line_ids 行角色与目标不一致: generation_spec_id={}, line_id={}, speaker_profile_id={:?}, expected={}",
            spec.id,
            line.id,
            line.speaker_profile_id,
            target_id
        );
        ensure_package!(
            line.expression.as_deref().map(str::trim) == Some(expression.as_str()),
            "项目族导入包 Galgame 立绘差分参数 source_line_ids 行 expression 与目标不一致: generation_spec_id={}, line_id={}, expression={:?}, expected={}",
            spec.id,
            line.id,
            line.expression,
            expression
        );
        ensure_package!(
            line.pose
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                == pose.as_deref(),
            "项目族导入包 Galgame 立绘差分参数 source_line_ids 行 pose 与目标不一致: generation_spec_id={}, line_id={}, pose={:?}, expected={:?}",
            spec.id,
            line.id,
            line.pose,
            pose
        );
        if let Some(position) = normalized_galgame_position(line) {
            ensure_package!(
                is_valid_galgame_position(position),
                "项目族导入包 Galgame 立绘差分参数 source_line_ids 行 position 无效: generation_spec_id={}, line_id={}, position={}",
                spec.id,
                line.id,
                position
            );
            source_positions.insert(position.to_string());
        }
        let scene = galgame_scenes.get(&line.scene_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 立绘差分参数 source_line_ids 行所属场不存在: generation_spec_id={}, line_id={}, scene_id={}",
                spec.id, line_id, line.scene_id
            )
        })?;
        validate_generation_spec_galgame_scene_context(
            spec,
            source_snapshot_id,
            scene,
            galgame_scripts,
        )?;
    }
    validate_galgame_portrait_position_samples(spec.id, &position_samples, &source_positions)?;
    Ok(())
}

fn validate_galgame_portrait_position_samples(
    spec_id: i64,
    position_samples: &[String],
    source_positions: &BTreeSet<String>,
) -> Result<(), String> {
    let mut sample_positions = BTreeSet::new();
    for position in position_samples {
        ensure_package!(
            is_valid_galgame_position(position),
            "项目族导入包 Galgame 立绘差分参数 position_samples 包含无效站位: generation_spec_id={}, position={}",
            spec_id,
            position
        );
        sample_positions.insert(position.clone());
    }
    ensure_package!(
        sample_positions == *source_positions,
        "项目族导入包 Galgame 立绘差分参数 position_samples 与 source_line_ids 行站位不一致: generation_spec_id={}, position_samples={:?}, source_positions={:?}",
        spec_id,
        sample_positions,
        source_positions
    );
    Ok(())
}

fn required_generation_spec_target_id(spec: &generation_spec::Model) -> Result<i64, String> {
    spec.target_entity_id.ok_or_else(|| {
        format!(
            "项目族导入包 Galgame 生成意图缺少 target_entity_id: generation_spec_id={}",
            spec.id
        )
    })
}

fn generation_spec_params_object(
    spec: &generation_spec::Model,
) -> Result<&serde_json::Map<String, Value>, String> {
    spec.params_json.as_object().ok_or_else(|| {
        format!(
            "项目族导入包生成意图参数必须是 JSON 对象: generation_spec_id={}",
            spec.id
        )
    })
}

fn required_params_object<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    spec_id: i64,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    object.get(key).and_then(Value::as_object).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {key} 必须是对象: generation_spec_id={spec_id}")
    })
}

fn required_params_i64(
    object: &serde_json::Map<String, Value>,
    path: &str,
    spec_id: i64,
) -> Result<i64, String> {
    let key = path.rsplit('.').next().unwrap_or(path);
    object.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {path} 必须是整数: generation_spec_id={spec_id}")
    })
}

fn optional_params_i64(
    object: &serde_json::Map<String, Value>,
    path: &str,
    spec_id: i64,
) -> Result<Option<i64>, String> {
    let key = path.rsplit('.').next().unwrap_or(path);
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value.as_i64().map(Some).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {path} 必须是整数: generation_spec_id={spec_id}")
    })
}

fn required_params_i64_array(
    object: &serde_json::Map<String, Value>,
    key: &str,
    spec_id: i64,
) -> Result<Vec<i64>, String> {
    let value = object.get(key).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {key} 缺失: generation_spec_id={spec_id}")
    })?;
    let items = value.as_array().ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {key} 必须是数组: generation_spec_id={spec_id}")
    })?;
    if items.is_empty() {
        return Err(format!(
            "项目族导入包 Galgame 生成意图参数 {key} 必须是非空数组: generation_spec_id={spec_id}"
        ));
    }
    let mut seen = BTreeSet::new();
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let id = item.as_i64().ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 生成意图参数 {key} 必须是整数数组: generation_spec_id={spec_id}"
            )
        })?;
        if id <= 0 {
            return Err(format!(
                "项目族导入包 Galgame 生成意图参数 {key} 必须是正整数数组: generation_spec_id={spec_id}"
            ));
        }
        if !seen.insert(id) {
            return Err(format!(
                "项目族导入包 Galgame 生成意图参数 {key} 不能包含重复 ID: generation_spec_id={spec_id}, id={id}"
            ));
        }
        ids.push(id);
    }
    Ok(ids)
}

fn required_params_string_array(
    object: &serde_json::Map<String, Value>,
    key: &str,
    spec_id: i64,
) -> Result<Vec<String>, String> {
    let value = object.get(key).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {key} 缺失: generation_spec_id={spec_id}")
    })?;
    let items = value.as_array().ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {key} 必须是数组: generation_spec_id={spec_id}")
    })?;
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let value = item
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 生成意图参数 {key} 必须是非空字符串数组: generation_spec_id={spec_id}"
                )
            })?;
        values.push(value.to_string());
    }
    Ok(values)
}

fn required_params_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    spec_id: i64,
) -> Result<String, String> {
    let value = object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 生成意图参数 {key} 必须是非空字符串: generation_spec_id={spec_id}"
            )
        })?;
    Ok(value.to_string())
}

fn optional_params_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    spec_id: i64,
) -> Result<Option<String>, String> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value.as_str().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame 生成意图参数 {key} 必须是字符串: generation_spec_id={spec_id}"
        )
    })?;
    let value = value.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value.to_string()))
    }
}

fn validate_optional_galgame_cg_importance(
    object: &serde_json::Map<String, Value>,
    spec_id: i64,
) -> Result<(), String> {
    let Some(importance) = optional_params_string(object, "cg_importance", spec_id)? else {
        return Ok(());
    };
    ensure_package!(
        GALGAME_CG_IMPORTANCE.contains(&importance.as_str()),
        "项目族导入包 Galgame 生成意图参数 cg_importance 无效: generation_spec_id={}, cg_importance={}",
        spec_id,
        importance
    );
    Ok(())
}

fn validate_generation_spec_canonical_key(
    spec: &generation_spec::Model,
    params: &serde_json::Map<String, Value>,
    expected_key: &str,
) -> Result<(), String> {
    let canonical_key = required_params_string(params, "canonical_key", spec.id)?;
    ensure_package!(
        canonical_key == expected_key,
        "项目族导入包 Galgame 生成意图 canonical_key 与目标不一致: generation_spec_id={}, canonical_key={}, expected={}",
        spec.id,
        canonical_key,
        expected_key
    );
    Ok(())
}

fn validate_params_id_equals(
    spec: &generation_spec::Model,
    object: &serde_json::Map<String, Value>,
    path: &str,
    expected_id: i64,
) -> Result<(), String> {
    let actual_id = required_params_i64(object, path, spec.id)?;
    ensure_package!(
        actual_id == expected_id,
        "项目族导入包 Galgame 生成意图参数 {path} 与目标不一致: generation_spec_id={}, actual={}, expected={}",
        spec.id,
        actual_id,
        expected_id
    );
    Ok(())
}

fn validate_snapshot_character_profile_id(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    profile_id: i64,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
) -> Result<(), String> {
    ensure_package!(
        snapshot_profiles.contains_key(&(
            source_snapshot_id,
            snapshot_profile_key("character_profile", profile_id),
        )),
        "项目族导入包 Galgame 生成意图参数角色不属于源快照: generation_spec_id={}, profile_id={}, source_snapshot_id={}",
        spec.id,
        profile_id,
        source_snapshot_id
    );
    Ok(())
}

fn validate_source_sync_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    let refs = SourceSyncReferenceSets::from_package(package);
    let runs = package
        .source_sync_runs
        .iter()
        .map(|run| (run.id, run))
        .collect::<HashMap<_, _>>();
    let conflicts = package
        .source_sync_conflicts
        .iter()
        .map(|conflict| (conflict.id, conflict))
        .collect::<HashMap<_, _>>();
    for run in &package.source_sync_runs {
        ensure_package!(
            is_valid_sync_run_mode(&run.mode),
            "项目族导入包源同步运行模式无效: source_sync_run_id={}, mode={}",
            run.id,
            run.mode
        );
        ensure_package!(
            is_valid_sync_run_status(&run.status),
            "项目族导入包源同步运行状态无效: source_sync_run_id={}, status={}",
            run.id,
            run.status
        );
        match run.status.as_str() {
            SYNC_RUN_STATUS_PLANNED if run.mode != SYNC_RUN_MODE_DRY_RUN => {
                return Err(format!(
                    "项目族导入包源同步运行状态与模式不一致: source_sync_run_id={}, status={}, mode={}",
                    run.id, run.status, run.mode
                ));
            }
            SYNC_RUN_STATUS_APPLYING | SYNC_RUN_STATUS_COMPLETED
                if run.mode != SYNC_RUN_MODE_APPLY =>
            {
                return Err(format!(
                    "项目族导入包源同步运行状态与模式不一致: source_sync_run_id={}, status={}, mode={}",
                    run.id, run.status, run.mode
                ));
            }
            _ => {}
        }
        ensure_package!(
            run.summary_json.is_object(),
            "项目族导入包源同步运行摘要必须是 JSON 对象: source_sync_run_id={}",
            run.id
        );
    }

    for conflict in &package.source_sync_conflicts {
        ensure_package!(
            is_valid_source_sync_conflict_target_ref(
                &conflict.conflict_scope,
                &conflict.target_kind,
            ),
            "项目族导入包源同步冲突目标类型组合无效: source_sync_conflict_id={}, conflict_scope={}, target_kind={}",
            conflict.id,
            conflict.conflict_scope,
            conflict.target_kind
        );
        ensure_package!(
            is_valid_source_change_kind(&conflict.source_change_kind),
            "项目族导入包源同步冲突源变更类型无效: source_sync_conflict_id={}, source_change_kind={}",
            conflict.id,
            conflict.source_change_kind
        );
        ensure_package!(
            is_valid_local_state_kind(&conflict.local_state_kind),
            "项目族导入包源同步冲突本地状态类型无效: source_sync_conflict_id={}, local_state_kind={}",
            conflict.id,
            conflict.local_state_kind
        );
        ensure_package!(
            is_valid_conflict_severity(&conflict.severity),
            "项目族导入包源同步冲突严重级别无效: source_sync_conflict_id={}, severity={}",
            conflict.id,
            conflict.severity
        );
        ensure_package!(
            is_valid_conflict_status(&conflict.status),
            "项目族导入包源同步冲突状态无效: source_sync_conflict_id={}, status={}",
            conflict.id,
            conflict.status
        );
        if let Some(strategy) = &conflict.resolution_strategy
            && !is_valid_resolution_strategy(strategy)
        {
            return Err(format!(
                "项目族导入包源同步冲突解决策略无效: source_sync_conflict_id={}, resolution_strategy={}",
                conflict.id, strategy
            ));
        }
        ensure_package!(
            conflict.source_summary_json.is_object(),
            "项目族导入包源同步冲突源摘要必须是 JSON 对象: source_sync_conflict_id={}",
            conflict.id
        );
        ensure_package!(
            conflict.local_summary_json.is_object(),
            "项目族导入包源同步冲突本地摘要必须是 JSON 对象: source_sync_conflict_id={}",
            conflict.id
        );
        ensure_package!(
            conflict.affected_refs_json.is_object(),
            "项目族导入包源同步冲突影响引用必须是 JSON 对象: source_sync_conflict_id={}",
            conflict.id
        );
        validate_source_sync_conflict_json_refs(conflict, &refs)?;
        match conflict.status.as_str() {
            CONFLICT_STATUS_PENDING
                if conflict.resolution_strategy.is_some()
                    || conflict.decision_json.is_some()
                    || conflict.resolved_at.is_some() =>
            {
                return Err(format!(
                    "项目族导入包待处理源同步冲突不能包含解决结果: source_sync_conflict_id={}",
                    conflict.id
                ));
            }
            CONFLICT_STATUS_RESOLVED | CONFLICT_STATUS_IGNORED
                if conflict.resolution_strategy.is_none()
                    || conflict.decision_json.is_none()
                    || conflict.resolved_at.is_none() =>
            {
                return Err(format!(
                    "项目族导入包已处理源同步冲突缺少解决结果: source_sync_conflict_id={}",
                    conflict.id
                ));
            }
            _ => {}
        }
        let run = runs.get(&conflict.sync_run_id).copied().ok_or_else(|| {
            format!(
                "项目族导入包源同步冲突引用了不存在的源同步运行: source_sync_conflict_id={}, sync_run_id={}",
                conflict.id, conflict.sync_run_id
            )
        })?;
        validate_source_sync_conflict_run_context(conflict, run)?;
        validate_source_sync_conflict_target_ref(conflict, &refs)?;
        if let (Some(strategy), Some(payload)) = (
            conflict.resolution_strategy.as_deref(),
            conflict.decision_json.as_ref(),
        ) {
            validate_source_sync_conflict_regenerate_scene_target(
                conflict, strategy, payload, &refs,
            )?;
            validate_source_sync_decision_payload(
                "source_sync_conflict",
                conflict.id,
                strategy,
                payload,
                &refs,
            )?;
        }
    }

    for decision in &package.source_sync_decisions {
        ensure_package!(
            is_valid_decision_actor(&decision.decided_by),
            "项目族导入包源同步决策人无效: source_sync_decision_id={}, decided_by={}",
            decision.id,
            decision.decided_by
        );
        ensure_package!(
            is_valid_resolution_strategy(&decision.strategy),
            "项目族导入包源同步决策策略无效: source_sync_decision_id={}, strategy={}",
            decision.id,
            decision.strategy
        );
        ensure_package!(
            decision.decision_payload_json.is_object(),
            "项目族导入包源同步决策载荷必须是 JSON 对象: source_sync_decision_id={}",
            decision.id
        );
        validate_source_sync_decision_conflict(decision, &conflicts)?;
        validate_source_sync_decision_payload(
            "source_sync_decision",
            decision.id,
            &decision.strategy,
            &decision.decision_payload_json,
            &refs,
        )?;
    }

    Ok(())
}

struct SourceSyncReferenceSets {
    generation_spec_ids: HashSet<i64>,
    source_snapshot_chapter_ids: HashSet<i64>,
    source_snapshot_chapter_numbers: HashSet<i32>,
    source_snapshot_chapter_contexts: HashMap<i64, (i64, i32)>,
    text_anchor_source_snapshot_chapter_ids: HashMap<i64, i64>,
    source_span_anchor_chapter_numbers: HashMap<i64, i32>,
    chapter_ids: HashSet<i64>,
    text_anchor_ids: HashSet<i64>,
    character_profile_ids: HashSet<i64>,
    object_profile_ids: HashSet<i64>,
    scene_profile_ids: HashSet<i64>,
    galgame_scene_ids: HashSet<i64>,
    galgame_scene_source_snapshot_chapter_ids: HashMap<i64, i64>,
    galgame_line_ids: HashSet<i64>,
    galgame_line_scene_ids: HashMap<i64, i64>,
}

impl SourceSyncReferenceSets {
    fn from_package(package: &ProjectFamilyPackage) -> Self {
        let snapshot_chapter_numbers = package
            .source_snapshot_chapters
            .iter()
            .map(|chapter| (chapter.id, chapter.chapter_number))
            .collect::<HashMap<_, _>>();
        Self {
            generation_spec_ids: package
                .generation_specs
                .iter()
                .map(|spec| spec.id)
                .collect(),
            source_snapshot_chapter_ids: package
                .source_snapshot_chapters
                .iter()
                .map(|chapter| chapter.id)
                .collect(),
            source_snapshot_chapter_numbers: package
                .source_snapshot_chapters
                .iter()
                .map(|chapter| chapter.chapter_number)
                .collect(),
            source_snapshot_chapter_contexts: package
                .source_snapshot_chapters
                .iter()
                .map(|chapter| {
                    (
                        chapter.id,
                        (chapter.source_chapter_id, chapter.chapter_number),
                    )
                })
                .collect(),
            text_anchor_source_snapshot_chapter_ids: package
                .text_anchors
                .iter()
                .filter_map(|anchor| {
                    anchor
                        .source_snapshot_chapter_id
                        .map(|chapter_id| (anchor.id, chapter_id))
                })
                .collect(),
            source_span_anchor_chapter_numbers: package
                .text_anchors
                .iter()
                .filter_map(|anchor| {
                    let snapshot_chapter_id = anchor.source_snapshot_chapter_id?;
                    let chapter_number = snapshot_chapter_numbers.get(&snapshot_chapter_id)?;
                    Some((anchor.id, *chapter_number))
                })
                .collect(),
            chapter_ids: package
                .text_core
                .chapters
                .iter()
                .map(|chapter| chapter.id)
                .collect(),
            text_anchor_ids: package
                .text_anchors
                .iter()
                .map(|anchor| anchor.id)
                .collect(),
            character_profile_ids: package
                .text_core
                .character_profiles
                .iter()
                .map(|profile| profile.id)
                .collect(),
            object_profile_ids: package
                .text_core
                .object_profiles
                .iter()
                .map(|profile| profile.id)
                .collect(),
            scene_profile_ids: package
                .text_core
                .scene_profiles
                .iter()
                .map(|profile| profile.id)
                .collect(),
            galgame_scene_ids: package
                .galgame_scenes
                .iter()
                .map(|scene| scene.id)
                .collect(),
            galgame_scene_source_snapshot_chapter_ids: package
                .galgame_scenes
                .iter()
                .map(|scene| (scene.id, scene.source_snapshot_chapter_id))
                .collect(),
            galgame_line_ids: package.galgame_lines.iter().map(|line| line.id).collect(),
            galgame_line_scene_ids: package
                .galgame_lines
                .iter()
                .map(|line| (line.id, line.scene_id))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RegenerateSceneStageOutputContext {
    expected_galgame_scene_id: i64,
    expected_source_snapshot_chapter_id: i64,
    expected_source_chapter_id: i64,
    expected_chapter_number: i32,
}

fn validate_source_sync_conflict_target_ref(
    conflict: &source_sync_conflict::Model,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let exists = match (
        conflict.conflict_scope.as_str(),
        conflict.target_kind.as_str(),
    ) {
        ("generation_spec", "segment") | ("text_anchor", "segment") => {
            refs.generation_spec_ids.contains(&conflict.target_id)
        }
        ("text_anchor", "chapter") => refs.chapter_ids.contains(&conflict.target_id),
        ("text_anchor", "galgame_scene") => refs.galgame_scene_ids.contains(&conflict.target_id),
        ("text_anchor", "galgame_line") => refs.galgame_line_ids.contains(&conflict.target_id),
        ("profile", "character") => refs.character_profile_ids.contains(&conflict.target_id),
        ("profile", "object") => refs.object_profile_ids.contains(&conflict.target_id),
        ("profile", "scene") => refs.scene_profile_ids.contains(&conflict.target_id),
        _ => return Ok(()),
    };
    ensure_package!(
        exists,
        "项目族导入包源同步冲突目标不存在: source_sync_conflict_id={}, conflict_scope={}, target_kind={}, target_id={}",
        conflict.id,
        conflict.conflict_scope,
        conflict.target_kind,
        conflict.target_id
    );
    Ok(())
}

fn validate_source_sync_conflict_json_refs(
    conflict: &source_sync_conflict::Model,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let source_summary = conflict
        .source_summary_json
        .as_object()
        .expect("source_summary_json object shape checked before ref validation");
    let local_summary = conflict
        .local_summary_json
        .as_object()
        .expect("local_summary_json object shape checked before ref validation");
    let affected_refs = conflict
        .affected_refs_json
        .as_object()
        .expect("affected_refs_json object shape checked before ref validation");

    if conflict.conflict_scope == "text_anchor" {
        validate_optional_source_sync_json_id_ref(
            source_summary,
            "old_text_anchor_id",
            &refs.text_anchor_ids,
            "文本锚点",
            "source_summary_json",
            conflict.id,
        )?;
        validate_optional_source_sync_json_id_ref(
            source_summary,
            "new_text_anchor_id",
            &refs.text_anchor_ids,
            "文本锚点",
            "source_summary_json",
            conflict.id,
        )?;
    }
    if conflict.conflict_scope == "profile" {
        validate_optional_source_sync_profile_ref(
            source_summary,
            "source_summary_json",
            conflict.id,
            refs,
        )?;
    }

    validate_optional_source_sync_json_id_ref(
        local_summary,
        "generation_spec_id",
        &refs.generation_spec_ids,
        "生成意图",
        "local_summary_json",
        conflict.id,
    )?;
    let local_galgame_scene_id = validate_optional_source_sync_json_id_ref(
        local_summary,
        "galgame_scene_id",
        &refs.galgame_scene_ids,
        "Galgame 场",
        "local_summary_json",
        conflict.id,
    )?;
    let local_galgame_line_id = validate_optional_source_sync_json_id_ref(
        local_summary,
        "galgame_line_id",
        &refs.galgame_line_ids,
        "Galgame 行",
        "local_summary_json",
        conflict.id,
    )?;
    validate_source_sync_galgame_line_scene_pair(
        local_galgame_line_id,
        local_galgame_scene_id,
        "local_summary_json",
        conflict.id,
        refs,
    )?;

    validate_optional_source_sync_json_id_ref(
        affected_refs,
        "generation_spec_id",
        &refs.generation_spec_ids,
        "生成意图",
        "affected_refs_json",
        conflict.id,
    )?;
    let affected_source_chapter_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "source_chapter_id",
        &refs.chapter_ids,
        "源章节",
        "affected_refs_json",
        conflict.id,
    )?;
    let affected_source_snapshot_chapter_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "source_snapshot_chapter_id",
        &refs.source_snapshot_chapter_ids,
        "源快照章节",
        "affected_refs_json",
        conflict.id,
    )?;
    let affected_old_source_snapshot_chapter_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "old_source_snapshot_chapter_id",
        &refs.source_snapshot_chapter_ids,
        "源快照章节",
        "affected_refs_json",
        conflict.id,
    )?;
    let affected_old_text_anchor_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "old_text_anchor_id",
        &refs.text_anchor_ids,
        "文本锚点",
        "affected_refs_json",
        conflict.id,
    )?;
    let affected_new_text_anchor_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "new_text_anchor_id",
        &refs.text_anchor_ids,
        "文本锚点",
        "affected_refs_json",
        conflict.id,
    )?;
    let affected_galgame_scene_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "galgame_scene_id",
        &refs.galgame_scene_ids,
        "Galgame 场",
        "affected_refs_json",
        conflict.id,
    )?;
    let affected_galgame_line_id = validate_optional_source_sync_json_id_ref(
        affected_refs,
        "galgame_line_id",
        &refs.galgame_line_ids,
        "Galgame 行",
        "affected_refs_json",
        conflict.id,
    )?;
    validate_source_sync_snapshot_chapter_source_pair(
        affected_source_snapshot_chapter_id,
        affected_source_chapter_id,
        "affected_refs_json",
        conflict.id,
        refs,
    )?;
    validate_source_sync_anchor_snapshot_chapter_pair(
        affected_old_text_anchor_id,
        affected_old_source_snapshot_chapter_id,
        "old_text_anchor_id",
        "old_source_snapshot_chapter_id",
        "affected_refs_json",
        conflict.id,
        refs,
    )?;
    validate_source_sync_anchor_snapshot_chapter_pair(
        affected_new_text_anchor_id,
        affected_source_snapshot_chapter_id,
        "new_text_anchor_id",
        "source_snapshot_chapter_id",
        "affected_refs_json",
        conflict.id,
        refs,
    )?;
    validate_source_sync_galgame_line_scene_pair(
        affected_galgame_line_id,
        affected_galgame_scene_id,
        "affected_refs_json",
        conflict.id,
        refs,
    )?;
    validate_optional_source_sync_profile_ref(
        affected_refs,
        "affected_refs_json",
        conflict.id,
        refs,
    )?;

    Ok(())
}

fn validate_optional_source_sync_json_id_ref(
    object: &serde_json::Map<String, Value>,
    key: &str,
    valid_ids: &HashSet<i64>,
    label: &str,
    json_label: &str,
    conflict_id: i64,
) -> Result<Option<i64>, String> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value.as_i64().ok_or_else(|| {
        format!(
            "项目族导入包源同步冲突 {json_label}.{key} 必须是整数: source_sync_conflict_id={conflict_id}"
        )
    })?;
    ensure_package!(
        valid_ids.contains(&value),
        "项目族导入包源同步冲突 {json_label}.{key} 引用的{label}不存在: source_sync_conflict_id={}, {key}={}",
        conflict_id,
        value
    );
    Ok(Some(value))
}

fn validate_source_sync_galgame_line_scene_pair(
    galgame_line_id: Option<i64>,
    galgame_scene_id: Option<i64>,
    json_label: &str,
    conflict_id: i64,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let (Some(line_id), Some(scene_id)) = (galgame_line_id, galgame_scene_id) else {
        return Ok(());
    };
    let line_scene_id = refs
        .galgame_line_scene_ids
        .get(&line_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包源同步冲突 {json_label}.galgame_line_id 引用的 Galgame 行不存在: source_sync_conflict_id={}, galgame_line_id={}",
                conflict_id, line_id
            )
        })?;
    ensure_package!(
        line_scene_id == scene_id,
        "项目族导入包源同步冲突 {json_label}.galgame_line_id 不属于 galgame_scene_id: source_sync_conflict_id={}, galgame_line_id={}, scene_id={}, expected={}",
        conflict_id,
        line_id,
        line_scene_id,
        scene_id
    );
    Ok(())
}

fn validate_source_sync_snapshot_chapter_source_pair(
    source_snapshot_chapter_id: Option<i64>,
    source_chapter_id: Option<i64>,
    json_label: &str,
    conflict_id: i64,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let (Some(snapshot_chapter_id), Some(source_chapter_id)) =
        (source_snapshot_chapter_id, source_chapter_id)
    else {
        return Ok(());
    };
    let (expected_source_chapter_id, _) = refs
        .source_snapshot_chapter_contexts
        .get(&snapshot_chapter_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包源同步冲突 {json_label}.source_snapshot_chapter_id 引用的源快照章节不存在: source_sync_conflict_id={}, source_snapshot_chapter_id={}",
                conflict_id, snapshot_chapter_id
            )
        })?;
    ensure_package!(
        source_chapter_id == expected_source_chapter_id,
        "项目族导入包源同步冲突 {json_label}.source_chapter_id 与 source_snapshot_chapter_id 不一致: source_sync_conflict_id={}, source_chapter_id={}, source_snapshot_chapter_id={}, expected_source_chapter_id={}",
        conflict_id,
        source_chapter_id,
        snapshot_chapter_id,
        expected_source_chapter_id
    );
    Ok(())
}

fn validate_source_sync_anchor_snapshot_chapter_pair(
    text_anchor_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
    anchor_key: &str,
    snapshot_chapter_key: &str,
    json_label: &str,
    conflict_id: i64,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let (Some(anchor_id), Some(snapshot_chapter_id)) = (text_anchor_id, source_snapshot_chapter_id)
    else {
        return Ok(());
    };
    let anchor_snapshot_chapter_id = refs
        .text_anchor_source_snapshot_chapter_ids
        .get(&anchor_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包源同步冲突 {json_label}.{anchor_key} 不属于源快照章节: source_sync_conflict_id={}, {anchor_key}={}",
                conflict_id, anchor_id
            )
        })?;
    ensure_package!(
        anchor_snapshot_chapter_id == snapshot_chapter_id,
        "项目族导入包源同步冲突 {json_label}.{anchor_key} 不属于 {snapshot_chapter_key}: source_sync_conflict_id={}, {anchor_key}={}, anchor_source_snapshot_chapter_id={}, {snapshot_chapter_key}={}",
        conflict_id,
        anchor_id,
        anchor_snapshot_chapter_id,
        snapshot_chapter_id
    );
    Ok(())
}

fn validate_optional_source_sync_profile_ref(
    object: &serde_json::Map<String, Value>,
    json_label: &str,
    conflict_id: i64,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let Some(value) = object.get("source_profile_id") else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let profile_kind = object
        .get("profile_kind")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "项目族导入包源同步冲突 {json_label}.profile_kind 必须是字符串: source_sync_conflict_id={conflict_id}"
            )
        })?;
    let (valid_ids, label) = match profile_kind {
        "character" => (&refs.character_profile_ids, "角色档案"),
        "object" => (&refs.object_profile_ids, "物体档案"),
        "scene" => (&refs.scene_profile_ids, "场景档案"),
        other => {
            return Err(format!(
                "项目族导入包源同步冲突 {json_label}.profile_kind 无效: source_sync_conflict_id={}, profile_kind={other}",
                conflict_id
            ));
        }
    };
    validate_optional_source_sync_json_id_ref(
        object,
        "source_profile_id",
        valid_ids,
        label,
        json_label,
        conflict_id,
    )?;
    Ok(())
}

fn validate_source_sync_conflict_regenerate_scene_target(
    conflict: &source_sync_conflict::Model,
    strategy: &str,
    payload: &Value,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    if strategy != "regenerate_scene" {
        return Ok(());
    }
    ensure_package!(
        conflict.conflict_scope == "text_anchor"
            && matches!(
                conflict.target_kind.as_str(),
                "galgame_scene" | "galgame_line"
            ),
        "项目族导入包 regenerate_scene 只能用于 Galgame 场/行文本锚点冲突: source_sync_conflict_id={}, conflict_scope={}, target_kind={}",
        conflict.id,
        conflict.conflict_scope,
        conflict.target_kind
    );
    let payload_scene_id = payload
        .get("galgame_scene_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene galgame_scene_id 必须是整数: source_sync_conflict_id={}",
                conflict.id
            )
        })?;
    let expected_scene_id = match conflict.target_kind.as_str() {
        "galgame_scene" => conflict.target_id,
        "galgame_line" => refs
            .galgame_line_scene_ids
            .get(&conflict.target_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包 regenerate_scene 冲突目标 Galgame 行不存在: source_sync_conflict_id={}, galgame_line_id={}",
                    conflict.id, conflict.target_id
                )
            })?,
        _ => unreachable!(),
    };
    ensure_package!(
        payload_scene_id == expected_scene_id,
        "项目族导入包 regenerate_scene galgame_scene_id 与冲突目标场不一致: source_sync_conflict_id={}, galgame_scene_id={}, expected={}",
        conflict.id,
        payload_scene_id,
        expected_scene_id
    );
    Ok(())
}

fn validate_source_sync_decision_payload(
    owner_kind: &str,
    owner_id: i64,
    strategy: &str,
    payload: &Value,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    if strategy != "regenerate_scene" {
        return Ok(());
    }
    validate_regenerate_scene_payload(owner_kind, owner_id, payload, refs)
}

fn validate_regenerate_scene_payload(
    owner_kind: &str,
    owner_id: i64,
    payload: &Value,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let object = payload.as_object().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene 决策载荷必须是 JSON 对象: {owner_kind}_id={owner_id}"
        )
    })?;
    validate_json_object_keys(
        object,
        &["user_action", "galgame_scene_id", "reason", "result"],
        "regenerate_scene 决策载荷",
        owner_kind,
        owner_id,
    )?;
    let expected_galgame_scene_id = validate_json_id_ref(
        object,
        "galgame_scene_id",
        &refs.galgame_scene_ids,
        "Galgame 场",
        owner_kind,
        owner_id,
    )?;
    validate_required_json_string_value(
        object,
        "user_action",
        "regenerate_scene",
        "regenerate_scene 决策载荷",
        owner_kind,
        owner_id,
    )?;
    validate_optional_json_string_or_null(
        object,
        "reason",
        "regenerate_scene 决策载荷",
        owner_kind,
        owner_id,
    )?;
    let result_value = object.get("result").ok_or_else(|| {
        format!("项目族导入包 regenerate_scene result 缺失: {owner_kind}_id={owner_id}")
    })?;
    let result = result_value.as_object().ok_or_else(|| {
        format!("项目族导入包 regenerate_scene result 必须是 JSON 对象: {owner_kind}_id={owner_id}")
    })?;
    validate_json_object_keys(
        result,
        &[
            "galgame_scene_id",
            "old_source_snapshot_chapter_id",
            "new_source_snapshot_chapter_id",
            "old_line_count",
            "new_line_count",
            "deleted_generation_spec_count",
            "deleted_portrait_generation_spec_count",
            "resolved_related_conflict_count",
            "mode",
            "stage_outputs",
        ],
        "regenerate_scene result",
        owner_kind,
        owner_id,
    )?;
    let result_scene_id = validate_required_result_json_id_ref(
        result,
        "galgame_scene_id",
        &refs.galgame_scene_ids,
        "Galgame 场",
        owner_kind,
        owner_id,
    )?;
    ensure_package!(
        result_scene_id == expected_galgame_scene_id,
        "项目族导入包 regenerate_scene result.galgame_scene_id 与决策目标场不一致: {owner_kind}_id={}, galgame_scene_id={}, expected={}",
        owner_id,
        result_scene_id,
        expected_galgame_scene_id
    );
    validate_required_result_json_id_ref(
        result,
        "old_source_snapshot_chapter_id",
        &refs.source_snapshot_chapter_ids,
        "源快照章节",
        owner_kind,
        owner_id,
    )?;
    let new_source_snapshot_chapter_id = validate_required_result_json_id_ref(
        result,
        "new_source_snapshot_chapter_id",
        &refs.source_snapshot_chapter_ids,
        "源快照章节",
        owner_kind,
        owner_id,
    )?;
    let actual_scene_snapshot_chapter_id = refs
        .galgame_scene_source_snapshot_chapter_ids
        .get(&expected_galgame_scene_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene galgame_scene_id 引用的 Galgame 场不存在: {owner_kind}_id={}, galgame_scene_id={}",
                owner_id, expected_galgame_scene_id
            )
        })?;
    ensure_package!(
        actual_scene_snapshot_chapter_id == new_source_snapshot_chapter_id,
        "项目族导入包 regenerate_scene result.new_source_snapshot_chapter_id 与 Galgame 场当前源快照章节不一致: {owner_kind}_id={}, galgame_scene_id={}, scene_source_snapshot_chapter_id={}, result_new_source_snapshot_chapter_id={}",
        owner_id,
        expected_galgame_scene_id,
        actual_scene_snapshot_chapter_id,
        new_source_snapshot_chapter_id
    );
    validate_required_non_negative_json_count(result, "old_line_count", owner_kind, owner_id)?;
    validate_required_non_negative_json_count(result, "new_line_count", owner_kind, owner_id)?;
    validate_required_non_negative_json_count(
        result,
        "deleted_generation_spec_count",
        owner_kind,
        owner_id,
    )?;
    validate_required_non_negative_json_count(
        result,
        "deleted_portrait_generation_spec_count",
        owner_kind,
        owner_id,
    )?;
    validate_required_non_negative_json_count(
        result,
        "resolved_related_conflict_count",
        owner_kind,
        owner_id,
    )?;
    validate_required_json_string_value(
        result,
        "mode",
        "llm_partial_regeneration",
        "regenerate_scene result",
        owner_kind,
        owner_id,
    )?;
    let stage_outputs = result.get("stage_outputs").ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene result.stage_outputs 缺失: {owner_kind}_id={owner_id}"
        )
    })?;
    if !stage_outputs.is_object() {
        return Err(format!(
            "项目族导入包 regenerate_scene result.stage_outputs 必须是 JSON 对象: {owner_kind}_id={owner_id}"
        ));
    }
    let (expected_source_chapter_id, expected_chapter_number) = refs
        .source_snapshot_chapter_contexts
        .get(&new_source_snapshot_chapter_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene result.new_source_snapshot_chapter_id 引用的源快照章节不存在: {owner_kind}_id={}, new_source_snapshot_chapter_id={}",
                owner_id, new_source_snapshot_chapter_id
            )
        })?;
    let context = RegenerateSceneStageOutputContext {
        expected_galgame_scene_id,
        expected_source_snapshot_chapter_id: new_source_snapshot_chapter_id,
        expected_source_chapter_id,
        expected_chapter_number,
    };
    validate_required_regenerate_scene_stage_outputs(stage_outputs, owner_kind, owner_id)?;
    validate_galgame_stage_outputs(
        stage_outputs,
        refs,
        &context,
        owner_kind,
        owner_id,
        "result.stage_outputs",
    )?;
    Ok(())
}

fn validate_required_regenerate_scene_stage_outputs(
    value: &Value,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let object = value.as_object().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene result.stage_outputs 必须是 JSON 对象: {owner_kind}_id={owner_id}"
        )
    })?;
    validate_json_object_keys(
        object,
        &[
            "scene_segmentation",
            "script_adaptation",
            "character_staging",
            "visual_planning",
            "branch_design",
        ],
        "regenerate_scene result.stage_outputs",
        owner_kind,
        owner_id,
    )?;
    for key in [
        "scene_segmentation",
        "script_adaptation",
        "character_staging",
        "visual_planning",
        "branch_design",
    ] {
        let Some(stage) = object.get(key) else {
            return Err(format!(
                "项目族导入包 regenerate_scene result.stage_outputs.{key} 缺失: {owner_kind}_id={owner_id}"
            ));
        };
        if !stage.is_object() {
            return Err(format!(
                "项目族导入包 regenerate_scene result.stage_outputs.{key} 必须是 JSON 对象: {owner_kind}_id={owner_id}"
            ));
        }
    }
    Ok(())
}

fn validate_galgame_stage_outputs(
    value: &Value,
    refs: &SourceSyncReferenceSets,
    context: &RegenerateSceneStageOutputContext,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            validate_regenerate_scene_segmentation_stage_output(
                object, owner_kind, owner_id, path,
            )?;
            validate_regenerate_scene_single_scene_line_stage_output(
                object, owner_kind, owner_id, path,
            )?;
            validate_regenerate_scene_visual_planning_stage_output(
                object, owner_kind, owner_id, path,
            )?;
            validate_regenerate_scene_branch_design_stage_output(
                object, owner_kind, owner_id, path,
            )?;
            if let Some(text_anchor_id) = validate_optional_json_id_ref(
                object,
                "text_anchor_id",
                &refs.text_anchor_ids,
                "文本锚点",
                owner_kind,
                owner_id,
            )? {
                let anchor_chapter_id = refs
                    .text_anchor_source_snapshot_chapter_ids
                    .get(&text_anchor_id)
                    .copied()
                    .ok_or_else(|| {
                        format!(
                            "项目族导入包 regenerate_scene {path}.text_anchor_id 不属于源快照章节: {owner_kind}_id={}, text_anchor_id={}",
                            owner_id, text_anchor_id
                        )
                    })?;
                ensure_package!(
                    anchor_chapter_id == context.expected_source_snapshot_chapter_id,
                    "项目族导入包 regenerate_scene {path}.text_anchor_id 不属于重建目标源快照章节: {owner_kind}_id={}, text_anchor_id={}, source_snapshot_chapter_id={}, expected={}",
                    owner_id,
                    text_anchor_id,
                    anchor_chapter_id,
                    context.expected_source_snapshot_chapter_id
                );
            }
            validate_optional_json_id_ref(
                object,
                "speaker_profile_id",
                &refs.character_profile_ids,
                "角色档案",
                owner_kind,
                owner_id,
            )?;
            validate_optional_json_id_ref(
                object,
                "scene_profile_id",
                &refs.scene_profile_ids,
                "场景档案",
                owner_kind,
                owner_id,
            )?;
            if let Some(source_snapshot_chapter_id) = validate_optional_json_id_ref(
                object,
                "source_snapshot_chapter_id",
                &refs.source_snapshot_chapter_ids,
                "源快照章节",
                owner_kind,
                owner_id,
            )? {
                ensure_package!(
                    source_snapshot_chapter_id == context.expected_source_snapshot_chapter_id,
                    "项目族导入包 regenerate_scene {path}.source_snapshot_chapter_id 与重建目标章节不一致: {owner_kind}_id={}, source_snapshot_chapter_id={}, expected={}",
                    owner_id,
                    source_snapshot_chapter_id,
                    context.expected_source_snapshot_chapter_id
                );
            }
            if let Some(source_chapter_id) = validate_optional_json_id_ref(
                object,
                "source_chapter_id",
                &refs.chapter_ids,
                "源章节",
                owner_kind,
                owner_id,
            )? {
                ensure_package!(
                    source_chapter_id == context.expected_source_chapter_id,
                    "项目族导入包 regenerate_scene {path}.source_chapter_id 与重建目标源章节不一致: {owner_kind}_id={}, source_chapter_id={}, expected={}",
                    owner_id,
                    source_chapter_id,
                    context.expected_source_chapter_id
                );
            }
            if let Some(galgame_scene_id) = validate_optional_json_id_ref(
                object,
                "galgame_scene_id",
                &refs.galgame_scene_ids,
                "Galgame 场",
                owner_kind,
                owner_id,
            )? {
                ensure_package!(
                    galgame_scene_id == context.expected_galgame_scene_id,
                    "项目族导入包 regenerate_scene {path}.galgame_scene_id 与重建目标场不一致: {owner_kind}_id={}, galgame_scene_id={}, expected={}",
                    owner_id,
                    galgame_scene_id,
                    context.expected_galgame_scene_id
                );
            }
            if let Some(galgame_line_id) = validate_optional_json_id_ref(
                object,
                "galgame_line_id",
                &refs.galgame_line_ids,
                "Galgame 行",
                owner_kind,
                owner_id,
            )? {
                let line_scene_id = refs
                    .galgame_line_scene_ids
                    .get(&galgame_line_id)
                    .copied()
                    .ok_or_else(|| {
                        format!(
                            "项目族导入包 regenerate_scene {path}.galgame_line_id 引用的 Galgame 行不存在: {owner_kind}_id={}, galgame_line_id={}",
                            owner_id, galgame_line_id
                        )
                    })?;
                ensure_package!(
                    line_scene_id == context.expected_galgame_scene_id,
                    "项目族导入包 regenerate_scene {path}.galgame_line_id 不属于重建目标场: {owner_kind}_id={}, galgame_line_id={}, scene_id={}, expected={}",
                    owner_id,
                    galgame_line_id,
                    line_scene_id,
                    context.expected_galgame_scene_id
                );
            }
            validate_source_span_ids(object, refs, context, owner_kind, owner_id, path)?;
            for (key, nested) in object {
                validate_galgame_stage_outputs(
                    nested,
                    refs,
                    context,
                    owner_kind,
                    owner_id,
                    &format!("{path}.{key}"),
                )?;
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                validate_galgame_stage_outputs(
                    item,
                    refs,
                    context,
                    owner_kind,
                    owner_id,
                    &format!("{path}[{index}]"),
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_regenerate_scene_segmentation_stage_output(
    object: &serde_json::Map<String, Value>,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    if !path.ends_with(".scene_segmentation") {
        return Ok(());
    }
    validate_json_object_keys(object, &["scenes"], path, owner_kind, owner_id)?;
    let scenes_value = object.get("scenes").ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.scenes 缺失: {owner_kind}_id={owner_id}")
    })?;
    let scenes = scenes_value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.scenes 必须是数组: {owner_kind}_id={owner_id}"
        )
    })?;
    ensure_package!(
        scenes.len() == 1,
        "项目族导入包 regenerate_scene {path}.scenes 必须且只能包含一个 index=1 场: {owner_kind}_id={}, scene_count={}",
        owner_id,
        scenes.len()
    );
    let scene = scenes.first().and_then(Value::as_object).ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.scenes[0] 必须是 JSON 对象: {owner_kind}_id={owner_id}"
        )
    })?;
    validate_required_stage_index(
        scene,
        "index",
        &format!("{path}.scenes[0]"),
        owner_kind,
        owner_id,
    )?;
    validate_required_nonempty_stage_source_span_ids(
        scene,
        &format!("{path}.scenes[0]"),
        owner_kind,
        owner_id,
    )?;
    Ok(())
}

fn validate_regenerate_scene_single_scene_line_stage_output(
    object: &serde_json::Map<String, Value>,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    if !path.ends_with(".script_adaptation") && !path.ends_with(".character_staging") {
        return Ok(());
    }
    validate_json_object_keys(object, &["scene_lines"], path, owner_kind, owner_id)?;
    let scene_lines_value = object.get("scene_lines").ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.scene_lines 缺失: {owner_kind}_id={owner_id}")
    })?;
    let scene_lines = scene_lines_value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.scene_lines 必须是数组: {owner_kind}_id={owner_id}"
        )
    })?;
    ensure_package!(
        scene_lines.len() == 1,
        "项目族导入包 regenerate_scene {path}.scene_lines 必须且只能包含一个 scene_index=1 场: {owner_kind}_id={}, scene_count={}",
        owner_id,
        scene_lines.len()
    );
    let scene = scene_lines.first().and_then(Value::as_object).ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.scene_lines[0] 必须是 JSON 对象: {owner_kind}_id={owner_id}"
        )
    })?;
    validate_required_stage_scene_index(
        scene,
        &format!("{path}.scene_lines[0]"),
        owner_kind,
        owner_id,
    )?;
    let lines = scene
        .get("lines")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene {path}.scene_lines[0].lines 必须是非空数组: {owner_kind}_id={owner_id}"
            )
        })?;
    ensure_package!(
        !lines.is_empty(),
        "项目族导入包 regenerate_scene {path}.scene_lines[0].lines 必须是非空数组: {owner_kind}_id={}",
        owner_id
    );
    for (line_index, line) in lines.iter().enumerate() {
        let line = line.as_object().ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene {path}.scene_lines[0].lines[{line_index}] 必须是 JSON 对象: {owner_kind}_id={owner_id}"
            )
        })?;
        validate_required_nonempty_stage_source_span_ids(
            line,
            &format!("{path}.scene_lines[0].lines[{line_index}]"),
            owner_kind,
            owner_id,
        )?;
    }
    Ok(())
}

fn validate_regenerate_scene_visual_planning_stage_output(
    object: &serde_json::Map<String, Value>,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    if !path.ends_with(".visual_planning") {
        return Ok(());
    }
    validate_json_object_keys(
        object,
        &["backgrounds", "event_cgs"],
        path,
        owner_kind,
        owner_id,
    )?;
    let backgrounds_value = object.get("backgrounds").ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.backgrounds 缺失: {owner_kind}_id={owner_id}")
    })?;
    let backgrounds = backgrounds_value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.backgrounds 必须是数组: {owner_kind}_id={owner_id}"
        )
    })?;
    ensure_package!(
        backgrounds.len() == 1,
        "项目族导入包 regenerate_scene {path}.backgrounds 必须且只能包含一个 scene_index=1 背景规划: {owner_kind}_id={}, background_count={}",
        owner_id,
        backgrounds.len()
    );
    let background = backgrounds.first().and_then(Value::as_object).ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.backgrounds[0] 必须是 JSON 对象: {owner_kind}_id={owner_id}"
        )
    })?;
    validate_required_stage_scene_index(
        background,
        &format!("{path}.backgrounds[0]"),
        owner_kind,
        owner_id,
    )?;
    validate_required_nonempty_stage_source_span_ids(
        background,
        &format!("{path}.backgrounds[0]"),
        owner_kind,
        owner_id,
    )?;
    let event_cgs_value = object.get("event_cgs").ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.event_cgs 缺失: {owner_kind}_id={owner_id}")
    })?;
    let event_cgs = event_cgs_value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.event_cgs 必须是数组: {owner_kind}_id={owner_id}"
        )
    })?;
    for (index, event_cg) in event_cgs.iter().enumerate() {
        let event_cg = event_cg.as_object().ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene {path}.event_cgs[{index}] 必须是 JSON 对象: {owner_kind}_id={owner_id}"
            )
        })?;
        validate_required_stage_scene_index(
            event_cg,
            &format!("{path}.event_cgs[{index}]"),
            owner_kind,
            owner_id,
        )?;
        validate_required_nonempty_stage_source_span_ids(
            event_cg,
            &format!("{path}.event_cgs[{index}]"),
            owner_kind,
            owner_id,
        )?;
    }
    Ok(())
}

fn validate_regenerate_scene_branch_design_stage_output(
    object: &serde_json::Map<String, Value>,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    if !path.ends_with(".branch_design") {
        return Ok(());
    }
    validate_json_object_keys(object, &["choices"], path, owner_kind, owner_id)?;
    let choices_value = object.get("choices").ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.choices 缺失: {owner_kind}_id={owner_id}")
    })?;
    let choices = choices_value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.choices 必须是数组: {owner_kind}_id={owner_id}"
        )
    })?;
    ensure_package!(
        choices.is_empty(),
        "项目族导入包 regenerate_scene {path}.choices 必须为空，局部重跑不允许新增选择支: {owner_kind}_id={}, choices={}",
        owner_id,
        choices.len()
    );
    Ok(())
}

fn validate_required_stage_scene_index(
    object: &serde_json::Map<String, Value>,
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    validate_required_stage_index(object, "scene_index", path, owner_kind, owner_id)
}

fn validate_required_stage_index(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let scene_index = object.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.{key} 必须是整数: {owner_kind}_id={owner_id}")
    })?;
    ensure_package!(
        scene_index == 1,
        "项目族导入包 regenerate_scene {path}.{key} 必须为 1: {owner_kind}_id={}, {key}={}",
        owner_id,
        scene_index
    );
    Ok(())
}

fn validate_required_nonempty_stage_source_span_ids(
    object: &serde_json::Map<String, Value>,
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let source_span_ids = object
        .get("source_span_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene {path}.source_span_ids 必须是非空字符串数组: {owner_kind}_id={owner_id}"
            )
        })?;
    ensure_package!(
        !source_span_ids.is_empty(),
        "项目族导入包 regenerate_scene {path}.source_span_ids 必须是非空字符串数组: {owner_kind}_id={}",
        owner_id
    );
    Ok(())
}

fn validate_json_object_keys(
    object: &serde_json::Map<String, Value>,
    allowed_keys: &[&str],
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    for key in object.keys() {
        ensure_package!(
            allowed_keys.contains(&key.as_str()),
            "项目族导入包 {path}.{key} 是未知字段: {owner_kind}_id={}",
            owner_id
        );
    }
    Ok(())
}

fn validate_required_json_string_value(
    object: &serde_json::Map<String, Value>,
    key: &str,
    expected: &str,
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let value = object.get(key).and_then(Value::as_str).ok_or_else(|| {
        format!("项目族导入包 {path}.{key} 必须是字符串: {owner_kind}_id={owner_id}")
    })?;
    ensure_package!(
        value == expected,
        "项目族导入包 {path}.{key} 无效: {owner_kind}_id={}, {key}={}",
        owner_id,
        value
    );
    Ok(())
}

fn validate_optional_json_string_or_null(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    if value.is_null() || value.is_string() {
        return Ok(());
    }
    Err(format!(
        "项目族导入包 {path}.{key} 必须是字符串或 null: {owner_kind}_id={owner_id}"
    ))
}

fn validate_source_span_ids(
    object: &serde_json::Map<String, Value>,
    refs: &SourceSyncReferenceSets,
    context: &RegenerateSceneStageOutputContext,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    let Some(value) = object.get("source_span_ids") else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let items = value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids 必须是字符串数组: {owner_kind}_id={owner_id}"
        )
    })?;
    for item in items {
        let span_id = item.as_str().ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene {path}.source_span_ids 必须是字符串数组: {owner_kind}_id={owner_id}"
            )
        })?;
        let (chapter_number, text_anchor_id) =
            parse_galgame_source_span_id(span_id, owner_kind, owner_id, path)?;
        ensure_package!(
            refs.source_snapshot_chapter_numbers
                .contains(&chapter_number),
            "项目族导入包 regenerate_scene {path}.source_span_ids 章节不存在: {owner_kind}_id={}, source_span_id={}",
            owner_id,
            span_id
        );
        ensure_package!(
            chapter_number == context.expected_chapter_number,
            "项目族导入包 regenerate_scene {path}.source_span_ids 章节编号与重建目标章节不一致: {owner_kind}_id={}, source_span_id={}, chapter_number={}, expected={}",
            owner_id,
            span_id,
            chapter_number,
            context.expected_chapter_number
        );
        ensure_package!(
            refs.text_anchor_ids.contains(&text_anchor_id),
            "项目族导入包 regenerate_scene {path}.source_span_ids 文本锚点不存在: {owner_kind}_id={}, source_span_id={}",
            owner_id,
            span_id
        );
        let actual_snapshot_chapter_id = refs
            .text_anchor_source_snapshot_chapter_ids
            .get(&text_anchor_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包 regenerate_scene {path}.source_span_ids 文本锚点不属于源快照章节: {owner_kind}_id={}, source_span_id={span_id}",
                    owner_id
                )
            })?;
        ensure_package!(
            actual_snapshot_chapter_id == context.expected_source_snapshot_chapter_id,
            "项目族导入包 regenerate_scene {path}.source_span_ids 文本锚点不属于重建目标源快照章节: {owner_kind}_id={}, source_span_id={}, source_snapshot_chapter_id={}, expected={}",
            owner_id,
            span_id,
            actual_snapshot_chapter_id,
            context.expected_source_snapshot_chapter_id
        );
        let actual_chapter_number = refs
            .source_span_anchor_chapter_numbers
            .get(&text_anchor_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包 regenerate_scene {path}.source_span_ids 文本锚点不属于源快照章节: {owner_kind}_id={}, source_span_id={span_id}",
                    owner_id
                )
            })?;
        ensure_package!(
            actual_chapter_number == chapter_number,
            "项目族导入包 regenerate_scene {path}.source_span_ids 章节编号与文本锚点不一致: {owner_kind}_id={}, source_span_id={}, anchor_chapter_number={}",
            owner_id,
            span_id,
            actual_chapter_number
        );
    }
    Ok(())
}

fn parse_galgame_source_span_id(
    span_id: &str,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(i32, i64), String> {
    let Some((chapter_prefix, anchor_id)) = span_id.rsplit_once(":ta") else {
        return Err(format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids 格式无效: {owner_kind}_id={}, source_span_id={span_id}",
            owner_id
        ));
    };
    let Some(chapter_number) = chapter_prefix.strip_prefix("ch") else {
        return Err(format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids 格式无效: {owner_kind}_id={}, source_span_id={span_id}",
            owner_id
        ));
    };
    let chapter_number = chapter_number.parse::<i32>().map_err(|_| {
        format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids 章节编号无效: {owner_kind}_id={}, source_span_id={span_id}",
            owner_id
        )
    })?;
    let text_anchor_id = anchor_id.parse::<i64>().map_err(|_| {
        format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids 文本锚点 ID 无效: {owner_kind}_id={}, source_span_id={span_id}",
            owner_id
        )
    })?;
    if chapter_number <= 0 || text_anchor_id <= 0 {
        return Err(format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids ID 必须为正数: {owner_kind}_id={}, source_span_id={span_id}",
            owner_id
        ));
    }
    Ok((chapter_number, text_anchor_id))
}

fn validate_json_id_ref(
    object: &serde_json::Map<String, Value>,
    key: &str,
    valid_ids: &HashSet<i64>,
    label: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<i64, String> {
    let value = object.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {key} 必须是整数: {owner_kind}_id={owner_id}")
    })?;
    ensure_package!(
        valid_ids.contains(&value),
        "项目族导入包 regenerate_scene {key} 引用的{label}不存在: {owner_kind}_id={}, {key}={}",
        owner_id,
        value
    );
    Ok(value)
}

fn validate_optional_json_id_ref(
    object: &serde_json::Map<String, Value>,
    key: &str,
    valid_ids: &HashSet<i64>,
    label: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<Option<i64>, String> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value.as_i64().ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {key} 必须是整数: {owner_kind}_id={owner_id}")
    })?;
    ensure_package!(
        valid_ids.contains(&value),
        "项目族导入包 regenerate_scene {key} 引用的{label}不存在: {owner_kind}_id={}, {key}={}",
        owner_id,
        value
    );
    Ok(Some(value))
}

fn validate_required_non_negative_json_count(
    object: &serde_json::Map<String, Value>,
    key: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<u64, String> {
    let value = object.get(key).ok_or_else(|| {
        format!("项目族导入包 regenerate_scene result.{key} 缺失: {owner_kind}_id={owner_id}")
    })?;
    value.as_u64().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene result.{key} 必须是非负整数: {owner_kind}_id={owner_id}"
        )
    })
}

fn validate_required_result_json_id_ref(
    object: &serde_json::Map<String, Value>,
    key: &str,
    valid_ids: &HashSet<i64>,
    label: &str,
    owner_kind: &str,
    owner_id: i64,
) -> Result<i64, String> {
    let value = object.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!("项目族导入包 regenerate_scene result.{key} 必须是整数: {owner_kind}_id={owner_id}")
    })?;
    ensure_package!(
        valid_ids.contains(&value),
        "项目族导入包 regenerate_scene result.{key} 引用的{label}不存在: {owner_kind}_id={}, {key}={}",
        owner_id,
        value
    );
    Ok(value)
}

fn is_valid_source_sync_conflict_target_ref(conflict_scope: &str, target_kind: &str) -> bool {
    matches!(
        (conflict_scope, target_kind),
        ("generation_spec", "segment")
            | ("text_anchor", "chapter")
            | ("text_anchor", "segment")
            | ("text_anchor", "galgame_scene")
            | ("text_anchor", "galgame_line")
            | ("profile", "character")
            | ("profile", "object")
            | ("profile", "scene")
    )
}

fn validate_source_sync_decision_conflict(
    decision: &source_sync_decision::Model,
    conflicts: &HashMap<i64, &source_sync_conflict::Model>,
) -> Result<(), String> {
    let conflict = conflicts.get(&decision.conflict_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包源同步决策引用了不存在的源同步冲突: source_sync_decision_id={}, conflict_id={}",
            decision.id, decision.conflict_id
        )
    })?;
    if conflict.status == CONFLICT_STATUS_PENDING {
        return Err(format!(
            "项目族导入包源同步决策引用了待处理源同步冲突: source_sync_decision_id={}, source_sync_conflict_id={}",
            decision.id, conflict.id
        ));
    }
    if conflict.resolution_strategy.as_deref() != Some(decision.strategy.as_str()) {
        return Err(format!(
            "项目族导入包源同步决策策略与冲突解决策略不一致: source_sync_decision_id={}, source_sync_conflict_id={}, decision_strategy={}, conflict_resolution_strategy={:?}",
            decision.id, conflict.id, decision.strategy, conflict.resolution_strategy
        ));
    }
    if conflict.decision_json.as_ref() != Some(&decision.decision_payload_json) {
        return Err(format!(
            "项目族导入包源同步决策载荷与冲突决策载荷不一致: source_sync_decision_id={}, source_sync_conflict_id={}",
            decision.id, conflict.id
        ));
    }
    Ok(())
}

fn validate_source_sync_conflict_run_context(
    conflict: &source_sync_conflict::Model,
    run: &source_sync_run::Model,
) -> Result<(), String> {
    if conflict.derived_project_id != run.derived_project_id
        || conflict.root_project_id != run.root_project_id
        || conflict.from_snapshot_id != run.from_snapshot_id
        || conflict.to_snapshot_id != run.to_snapshot_id
    {
        return Err(format!(
            "项目族导入包源同步冲突与所属运行上下文不一致: source_sync_conflict_id={}, sync_run_id={}, conflict=(derived_project_id={}, root_project_id={}, from_snapshot_id={}, to_snapshot_id={}), run=(derived_project_id={}, root_project_id={}, from_snapshot_id={}, to_snapshot_id={})",
            conflict.id,
            conflict.sync_run_id,
            conflict.derived_project_id,
            conflict.root_project_id,
            conflict.from_snapshot_id,
            conflict.to_snapshot_id,
            run.derived_project_id,
            run.root_project_id,
            run.from_snapshot_id,
            run.to_snapshot_id
        ));
    }
    Ok(())
}

pub(super) fn validate_media_manifest(
    package: &ProjectFamilyPackage,
    media_payload: &ImportMediaPayload,
) -> Result<(), String> {
    let media_asset_ids = package
        .media_assets
        .iter()
        .map(|asset| asset.id)
        .collect::<HashSet<_>>();
    let media_assets_by_id = package
        .media_assets
        .iter()
        .map(|asset| (asset.id, asset))
        .collect::<HashMap<_, _>>();
    let generation_specs = package
        .generation_specs
        .iter()
        .map(|spec| (spec.id, spec))
        .collect::<HashMap<_, _>>();
    let prompt_artifacts = package
        .prompt_artifacts
        .iter()
        .map(|artifact| (artifact.id, artifact))
        .collect::<HashMap<_, _>>();
    let chapter_ids = package
        .text_core
        .chapters
        .iter()
        .map(|chapter| chapter.id)
        .collect::<HashSet<_>>();
    let character_profile_ids = package
        .text_core
        .character_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    let object_profile_ids = package
        .text_core
        .object_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    let scene_profile_ids = package
        .text_core
        .scene_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    let video_segment_ids = package
        .video_segments
        .iter()
        .map(|segment| segment.id)
        .collect::<HashSet<_>>();
    let assembly_timeline_ids = package
        .assembly_timelines
        .iter()
        .map(|timeline| timeline.id)
        .collect::<HashSet<_>>();
    let galgame_scene_ids = package
        .galgame_scenes
        .iter()
        .map(|scene| scene.id)
        .collect::<HashSet<_>>();
    let galgame_line_ids = package
        .galgame_lines
        .iter()
        .map(|line| line.id)
        .collect::<HashSet<_>>();
    let galgame_scripts = package
        .galgame_scripts
        .iter()
        .map(|script| (script.id, script))
        .collect::<HashMap<_, _>>();
    let galgame_scenes = package
        .galgame_scenes
        .iter()
        .map(|scene| (scene.id, scene))
        .collect::<HashMap<_, _>>();
    let galgame_lines = package
        .galgame_lines
        .iter()
        .map(|line| (line.id, line))
        .collect::<HashMap<_, _>>();
    let generation_spec_ids = generation_specs.keys().copied().collect::<HashSet<_>>();
    let prompt_artifact_ids = prompt_artifacts.keys().copied().collect::<HashSet<_>>();
    let media_request_refs = MediaRequestRefContext {
        generation_specs: &generation_specs,
        generation_spec_ids: &generation_spec_ids,
        prompt_artifacts: &prompt_artifacts,
        prompt_artifact_ids: &prompt_artifact_ids,
        character_profile_ids: &character_profile_ids,
        video_segment_ids: &video_segment_ids,
        assembly_timeline_ids: &assembly_timeline_ids,
        galgame_scripts: &galgame_scripts,
        galgame_scenes: &galgame_scenes,
        galgame_lines: &galgame_lines,
    };
    let media_provenance_refs = MediaProvenanceRefContext {
        media_assets: &media_assets_by_id,
        request: media_request_refs,
    };
    let source_snapshots = package
        .source_snapshots
        .iter()
        .map(|snapshot| (snapshot.id, snapshot))
        .collect::<HashMap<_, _>>();
    let source_snapshot_chapters = package
        .source_snapshot_chapters
        .iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<HashMap<_, _>>();
    for asset in &package.media_assets {
        if asset.project_id == package.root_project_id {
            return Err(format!(
                "项目族导入包媒体资产不能属于根文本项目: media_asset_id={}",
                asset.id
            ));
        }
        if !is_valid_media_status(&asset.status) {
            return Err(format!(
                "项目族导入包媒体资产状态无效: media_asset_id={}, status={}",
                asset.id, asset.status
            ));
        }
        if !is_valid_media_kind(&asset.media_kind) {
            return Err(format!(
                "项目族导入包媒体资产类型无效: media_asset_id={}, media_kind={}",
                asset.id, asset.media_kind
            ));
        }
        if !is_valid_media_origin(&asset.origin) {
            return Err(format!(
                "项目族导入包媒体资产来源无效: media_asset_id={}, origin={}",
                asset.id, asset.origin
            ));
        }
        if asset.root_project_id != package.root_project_id {
            return Err(format!(
                "媒体资产 root_project_id 与导入包根项目不一致: media_asset_id={}",
                asset.id
            ));
        }
        validate_media_asset_source_context(
            asset,
            package.root_project_id,
            &source_snapshots,
            &source_snapshot_chapters,
        )?;
        if media_asset_path(asset, MEDIA_FILE_FIELD_FILE)?.is_some()
            && !media_payload
                .entries
                .contains_key(&media_file_key(asset.id, MEDIA_FILE_FIELD_FILE))
        {
            return Err(format!("媒体资产缺少文件包条目: {}", asset.id));
        }
        if media_asset_path(asset, MEDIA_FILE_FIELD_THUMBNAIL)?.is_some()
            && !media_payload
                .entries
                .contains_key(&media_file_key(asset.id, MEDIA_FILE_FIELD_THUMBNAIL))
        {
            return Err(format!("媒体资产缺少缩略图包条目: {}", asset.id));
        }
        if asset.status == MEDIA_STATUS_COMPLETED && asset.file_path.is_none() {
            return Err(format!("completed 媒体资产缺少 file_path: {}", asset.id));
        }
    }
    for provenance in &package.media_provenance {
        let asset = media_assets_by_id
            .get(&provenance.media_asset_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包媒体来源引用了不存在的媒体资产: media_asset_id={}",
                    provenance.media_asset_id
                )
            })?;
        validate_media_provenance_refs(provenance, asset, &media_provenance_refs)?;
    }
    for binding in &package.media_bindings {
        let asset = media_assets_by_id.get(&binding.media_asset_id);
        if asset.is_none() {
            return Err(format!(
                "项目族导入包媒体绑定引用了不存在的媒体资产: media_binding_id={}, media_asset_id={}",
                binding.id, binding.media_asset_id
            ));
        }
        if !is_valid_media_binding_target_kind(&binding.target_kind) {
            return Err(format!(
                "项目族导入包媒体绑定目标类型无效: media_binding_id={}, target_kind={}",
                binding.id, binding.target_kind
            ));
        }
        if !is_valid_media_binding_role(&binding.role) {
            return Err(format!(
                "项目族导入包媒体绑定角色无效: media_binding_id={}, role={}",
                binding.id, binding.role
            ));
        }
        validate_media_binding_target_ref(
            binding,
            &chapter_ids,
            &character_profile_ids,
            &object_profile_ids,
            &scene_profile_ids,
            &video_segment_ids,
            &assembly_timeline_ids,
            &galgame_scene_ids,
            &galgame_line_ids,
        )?;
        validate_galgame_media_binding_contract(
            binding,
            asset.expect("checked media asset exists"),
            &galgame_scripts,
            &galgame_scenes,
            &galgame_lines,
        )?;
    }
    for entry in media_payload.entries.values() {
        if !media_asset_ids.contains(&entry.media_asset_id) {
            return Err(format!(
                "媒体文件条目引用了不存在的媒体资产: {}",
                entry.media_asset_id
            ));
        }
    }
    Ok(())
}

fn validate_media_asset_source_context(
    asset: &media_asset::Model,
    root_project_id: i64,
    source_snapshots: &HashMap<i64, &project_source_snapshot::Model>,
    source_snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
) -> Result<(), String> {
    let source_snapshot_id = asset.source_snapshot_id.ok_or_else(|| {
        format!(
            "项目族导入包媒体资产缺少 source_snapshot_id: media_asset_id={}",
            asset.id
        )
    })?;
    let snapshot = source_snapshots.get(&source_snapshot_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包媒体资产引用了不存在的源文本快照: media_asset_id={}, source_snapshot_id={source_snapshot_id}",
            asset.id
        )
    })?;
    if snapshot.root_project_id != root_project_id {
        return Err(format!(
            "项目族导入包媒体资产 source_snapshot_id 不属于根项目: media_asset_id={}, source_snapshot_id={source_snapshot_id}",
            asset.id
        ));
    }
    if asset.source_chapter_id.is_some() != asset.source_snapshot_chapter_id.is_some() {
        return Err(format!(
            "项目族导入包媒体资产 source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空: media_asset_id={}",
            asset.id
        ));
    }
    if let Some(source_snapshot_chapter_id) = asset.source_snapshot_chapter_id {
        let snapshot_chapter = source_snapshot_chapters
            .get(&source_snapshot_chapter_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包媒体资产引用了不存在的源快照章节: media_asset_id={}, source_snapshot_chapter_id={source_snapshot_chapter_id}",
                    asset.id
                )
            })?;
        if snapshot_chapter.snapshot_id != source_snapshot_id {
            return Err(format!(
                "项目族导入包媒体资产源快照章节不属于目标快照: media_asset_id={}, source_snapshot_id={}, source_snapshot_chapter_id={}",
                asset.id, source_snapshot_id, source_snapshot_chapter_id
            ));
        }
        let source_chapter_id = asset
            .source_chapter_id
            .expect("media asset chapter context is checked as paired");
        if snapshot_chapter.source_chapter_id != source_chapter_id {
            return Err(format!(
                "项目族导入包媒体资产 source_chapter_id 与 source_snapshot_chapter_id 不一致: media_asset_id={}, source_chapter_id={}, source_snapshot_chapter_id={}, expected_source_chapter_id={}",
                asset.id,
                source_chapter_id,
                source_snapshot_chapter_id,
                snapshot_chapter.source_chapter_id
            ));
        }
    }
    Ok(())
}

fn validate_galgame_media_binding_contract(
    binding: &media_binding::Model,
    asset: &media_asset::Model,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
    galgame_lines: &HashMap<i64, &galgame_line::Model>,
) -> Result<(), String> {
    if !is_valid_galgame_media_binding_pair(&binding.target_kind, &binding.role) {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定目标/角色无效: media_binding_id={}, target_kind={}, role={}",
            binding.id, binding.target_kind, binding.role
        ));
    }
    if let Some(required_media_kind) =
        galgame_media_binding_required_media_kind(&binding.target_kind, &binding.role)
        && asset.media_kind != required_media_kind
    {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定资产类型不匹配: media_binding_id={}, media_asset_id={}, target_kind={}, role={}, required_media_kind={}, actual_media_kind={}",
            binding.id,
            asset.id,
            binding.target_kind,
            binding.role,
            required_media_kind,
            asset.media_kind
        ));
    }
    if matches!(
        binding.target_kind.as_str(),
        "galgame_scene" | "galgame_line"
    ) && (asset.status != MEDIA_STATUS_COMPLETED || asset.file_path.is_none())
    {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定资产不是可用已完成媒体: media_binding_id={}, media_asset_id={}",
            binding.id, asset.id
        ));
    }
    match binding.target_kind.as_str() {
        "galgame_scene" => {
            let scene = galgame_scenes.get(&binding.target_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 媒体绑定目标场不存在: media_binding_id={}, target_id={}",
                    binding.id, binding.target_id
                )
            })?;
            validate_galgame_media_binding_project(binding, asset, scene, galgame_scripts)?;
        }
        "galgame_line" => {
            let line = galgame_lines.get(&binding.target_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 媒体绑定目标行不存在: media_binding_id={}, target_id={}",
                    binding.id, binding.target_id
                )
            })?;
            let scene = galgame_scenes.get(&line.scene_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 媒体绑定目标行所属场不存在: media_binding_id={}, target_id={}, scene_id={}",
                    binding.id, binding.target_id, line.scene_id
                )
            })?;
            validate_galgame_media_binding_project(binding, asset, scene, galgame_scripts)?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_galgame_media_binding_project(
    binding: &media_binding::Model,
    asset: &media_asset::Model,
    scene: &galgame_scene::Model,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
) -> Result<(), String> {
    let script = galgame_scripts.get(&scene.script_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame 媒体绑定目标场引用了不存在的剧本: media_binding_id={}, scene_id={}, script_id={}",
            binding.id, scene.id, scene.script_id
        )
    })?;
    if asset.project_id != script.project_id || asset.root_project_id != script.root_project_id {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定资产与目标项目不一致: media_binding_id={}, media_asset_id={}, asset_project_id={}, target_project_id={}",
            binding.id, asset.id, asset.project_id, script.project_id
        ));
    }
    if asset.source_snapshot_id != Some(script.source_snapshot_id) {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定资产源快照与目标不一致: media_binding_id={}, media_asset_id={}, asset_source_snapshot_id={:?}, target_source_snapshot_id={}",
            binding.id, asset.id, asset.source_snapshot_id, script.source_snapshot_id
        ));
    }
    if let Some(source_snapshot_chapter_id) = asset.source_snapshot_chapter_id
        && source_snapshot_chapter_id != scene.source_snapshot_chapter_id
    {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定资产源快照章节与目标不一致: media_binding_id={}, media_asset_id={}, asset_source_snapshot_chapter_id={}, target_source_snapshot_chapter_id={}",
            binding.id, asset.id, source_snapshot_chapter_id, scene.source_snapshot_chapter_id
        ));
    }
    Ok(())
}

struct MediaRequestRefContext<'a> {
    generation_specs: &'a HashMap<i64, &'a generation_spec::Model>,
    generation_spec_ids: &'a HashSet<i64>,
    prompt_artifacts: &'a HashMap<i64, &'a prompt_artifact::Model>,
    prompt_artifact_ids: &'a HashSet<i64>,
    character_profile_ids: &'a HashSet<i64>,
    video_segment_ids: &'a HashSet<i64>,
    assembly_timeline_ids: &'a HashSet<i64>,
    galgame_scripts: &'a HashMap<i64, &'a galgame_script::Model>,
    galgame_scenes: &'a HashMap<i64, &'a galgame_scene::Model>,
    galgame_lines: &'a HashMap<i64, &'a galgame_line::Model>,
}

struct MediaProvenanceRefContext<'a> {
    media_assets: &'a HashMap<i64, &'a media_asset::Model>,
    request: MediaRequestRefContext<'a>,
}

fn validate_media_provenance_refs(
    provenance: &media_provenance::Model,
    asset: &media_asset::Model,
    refs: &MediaProvenanceRefContext<'_>,
) -> Result<(), String> {
    let linked_spec = if let Some(spec_id) = provenance.generation_spec_id {
        let spec = refs
            .request
            .generation_specs
            .get(&spec_id)
            .copied()
            .ok_or_else(|| {
            format!(
                "项目族导入包媒体来源 generation_spec_id 不存在: media_asset_id={}, generation_spec_id={spec_id}",
                provenance.media_asset_id
            )
        })?;
        if asset.project_id != spec.project_id || asset.root_project_id != spec.root_project_id {
            return Err(format!(
                "项目族导入包媒体来源生成意图与媒体资产项目不一致: media_asset_id={}, generation_spec_id={spec_id}",
                provenance.media_asset_id
            ));
        }
        validate_media_provenance_source_context(
            provenance.media_asset_id,
            asset,
            spec.source_snapshot_id,
            spec.source_snapshot_chapter_id,
            spec.source_chapter_id,
            "generation_spec_id",
            spec.id,
        )?;
        Some(spec)
    } else {
        None
    };

    if let Some(prompt_id) = provenance.prompt_artifact_id {
        let prompt = refs
            .request
            .prompt_artifacts
            .get(&prompt_id)
            .copied()
            .ok_or_else(|| {
            format!(
                "项目族导入包媒体来源 prompt_artifact_id 不存在: media_asset_id={}, prompt_artifact_id={prompt_id}",
                provenance.media_asset_id
            )
        })?;
        if let Some(spec) = linked_spec
            && prompt.generation_spec_id != spec.id
        {
            return Err(format!(
                "项目族导入包媒体来源 prompt_artifact_id 与 generation_spec_id 不一致: media_asset_id={}, prompt_artifact_id={prompt_id}, generation_spec_id={}",
                provenance.media_asset_id, spec.id
            ));
        }
        if linked_spec.is_none() {
            let prompt_spec = refs
                .request
                .generation_specs
                .get(&prompt.generation_spec_id)
                .copied()
                .ok_or_else(|| {
                    format!(
                        "项目族导入包媒体来源 prompt_artifact 关联的生成意图不存在: media_asset_id={}, prompt_artifact_id={prompt_id}, generation_spec_id={}",
                        provenance.media_asset_id, prompt.generation_spec_id
                    )
                })?;
            validate_media_provenance_source_context(
                provenance.media_asset_id,
                asset,
                prompt_spec.source_snapshot_id,
                prompt_spec.source_snapshot_chapter_id,
                prompt_spec.source_chapter_id,
                "prompt_artifact.generation_spec_id",
                prompt_spec.id,
            )?;
        }
    }

    if let Some(source_media_asset_id) = provenance.source_media_asset_id {
        let source_asset = refs
            .media_assets
            .get(&source_media_asset_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包媒体来源 source_media_asset_id 不存在: media_asset_id={}, source_media_asset_id={source_media_asset_id}",
                    provenance.media_asset_id
                )
            })?;
        if source_asset.root_project_id != asset.root_project_id {
            return Err(format!(
                "项目族导入包媒体来源 source_media_asset_id 与媒体资产根项目不一致: media_asset_id={}, source_media_asset_id={source_media_asset_id}",
                provenance.media_asset_id
            ));
        }
        if source_asset.status != MEDIA_STATUS_COMPLETED || source_asset.file_path.is_none() {
            return Err(format!(
                "项目族导入包媒体来源 source_media_asset_id 不是可追溯的已完成媒体: media_asset_id={}, source_media_asset_id={source_media_asset_id}",
                provenance.media_asset_id
            ));
        }
        validate_media_provenance_source_context(
            provenance.media_asset_id,
            asset,
            source_asset.source_snapshot_id,
            source_asset.source_snapshot_chapter_id,
            source_asset.source_chapter_id,
            "source_media_asset_id",
            source_asset.id,
        )?;
    }

    let Some(request_snapshot) = provenance.request_snapshot_json.as_ref() else {
        if linked_spec.is_some_and(is_galgame_visual_generation_spec) {
            return Err(format!(
                "项目族导入包 Galgame 资产媒体来源缺少 request_snapshot_json: media_asset_id={}",
                provenance.media_asset_id
            ));
        }
        return Ok(());
    };
    let request = request_snapshot.as_object().ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json 必须是 JSON 对象: media_asset_id={}",
            provenance.media_asset_id
        )
    })?;
    validate_media_request_snapshot_refs(provenance, asset, linked_spec, request, &refs.request)
}

fn validate_media_provenance_source_context(
    media_asset_id: i64,
    asset: &media_asset::Model,
    ref_source_snapshot_id: Option<i64>,
    ref_source_snapshot_chapter_id: Option<i64>,
    ref_source_chapter_id: Option<i64>,
    label: &str,
    ref_id: i64,
) -> Result<(), String> {
    match asset.source_snapshot_id {
        Some(asset_snapshot_id) if ref_source_snapshot_id != Some(asset_snapshot_id) => {
            return Err(format!(
                "项目族导入包媒体来源 {label} 与媒体资产 source_snapshot_id 不一致: media_asset_id={media_asset_id}, {label}={ref_id}, asset_source_snapshot_id={asset_snapshot_id}, ref_source_snapshot_id={ref_source_snapshot_id:?}"
            ));
        }
        None if ref_source_snapshot_id.is_some() || ref_source_snapshot_chapter_id.is_some() => {
            return Err(format!(
                "项目族导入包 freeform 媒体来源不能带源快照上下文: media_asset_id={media_asset_id}, {label}={ref_id}"
            ));
        }
        _ => {}
    }

    if let Some(asset_snapshot_chapter_id) = asset.source_snapshot_chapter_id
        && ref_source_snapshot_chapter_id != Some(asset_snapshot_chapter_id)
    {
        return Err(format!(
            "项目族导入包媒体来源 {label} 与媒体资产 source_snapshot_chapter_id 不一致: media_asset_id={media_asset_id}, {label}={ref_id}, asset_source_snapshot_chapter_id={asset_snapshot_chapter_id}, ref_source_snapshot_chapter_id={ref_source_snapshot_chapter_id:?}"
        ));
    }
    if let Some(asset_source_chapter_id) = asset.source_chapter_id
        && ref_source_chapter_id != Some(asset_source_chapter_id)
    {
        return Err(format!(
            "项目族导入包媒体来源 {label} 与媒体资产 source_chapter_id 不一致: media_asset_id={media_asset_id}, {label}={ref_id}, asset_source_chapter_id={asset_source_chapter_id}, ref_source_chapter_id={ref_source_chapter_id:?}"
        ));
    }
    Ok(())
}

fn validate_media_request_snapshot_refs(
    provenance: &media_provenance::Model,
    asset: &media_asset::Model,
    linked_spec: Option<&generation_spec::Model>,
    request: &serde_json::Map<String, Value>,
    refs: &MediaRequestRefContext<'_>,
) -> Result<(), String> {
    validate_optional_media_request_id_ref(
        request,
        "generation_spec_id",
        refs.generation_spec_ids,
        "生成意图",
        provenance.media_asset_id,
    )?;
    validate_optional_media_request_id_ref(
        request,
        "prompt_artifact_id",
        refs.prompt_artifact_ids,
        "提示词产物",
        provenance.media_asset_id,
    )?;
    validate_optional_media_request_id_ref(
        request,
        "timeline_id",
        refs.assembly_timeline_ids,
        "成片时间线",
        provenance.media_asset_id,
    )?;

    let request_source = request.get("source").and_then(Value::as_str).map(str::trim);
    if request_source == Some("galgame_audio_import") {
        let target_line_id =
            required_media_request_i64(request, "target_line_id", provenance.media_asset_id)?;
        validate_media_request_galgame_target_line(
            provenance.media_asset_id,
            asset,
            target_line_id,
            refs.galgame_scripts,
            refs.galgame_scenes,
            refs.galgame_lines,
        )?;
    } else if let Some(target_line_id) =
        optional_media_request_i64(request, "target_line_id", provenance.media_asset_id)?
    {
        validate_media_request_galgame_target_line(
            provenance.media_asset_id,
            asset,
            target_line_id,
            refs.galgame_scripts,
            refs.galgame_scenes,
            refs.galgame_lines,
        )?;
    }

    let is_galgame_asset_generation = request_source == Some("galgame_asset_generation");
    if linked_spec.is_some_and(is_galgame_visual_generation_spec) && !is_galgame_asset_generation {
        return Err(format!(
            "项目族导入包 Galgame 资产媒体来源 request_snapshot_json.source 必须是 galgame_asset_generation: media_asset_id={}",
            provenance.media_asset_id
        ));
    }

    if is_galgame_asset_generation {
        let spec = linked_spec.ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 资产媒体来源缺少 generation_spec_id: media_asset_id={}",
                provenance.media_asset_id
            )
        })?;
        let target_kind =
            required_media_request_string(request, "target_kind", provenance.media_asset_id)?;
        let target_entity_kind = required_media_request_string(
            request,
            "target_entity_kind",
            provenance.media_asset_id,
        )?;
        let target_entity_id =
            required_media_request_i64(request, "target_entity_id", provenance.media_asset_id)?;
        validate_media_request_target_entity(
            provenance.media_asset_id,
            asset,
            target_entity_kind,
            target_entity_id,
            refs,
        )?;
        if target_kind != spec.target_kind
            || target_entity_kind != spec.target_entity_kind
            || spec.target_entity_id != Some(target_entity_id)
        {
            return Err(format!(
                "项目族导入包 Galgame 资产媒体来源 request_snapshot_json 与 generation_spec 目标不一致: media_asset_id={}, generation_spec_id={}",
                provenance.media_asset_id, spec.id
            ));
        }
    } else if let Some(target_entity_id) =
        optional_media_request_i64(request, "target_entity_id", provenance.media_asset_id)?
    {
        let target_entity_kind = required_media_request_string(
            request,
            "target_entity_kind",
            provenance.media_asset_id,
        )?;
        validate_media_request_target_entity(
            provenance.media_asset_id,
            asset,
            target_entity_kind,
            target_entity_id,
            refs,
        )?;
    }
    Ok(())
}

fn is_galgame_visual_generation_spec(spec: &generation_spec::Model) -> bool {
    matches!(
        (spec.target_kind.as_str(), spec.target_entity_kind.as_str()),
        ("galgame_background", "galgame_scene") | ("galgame_cg", "galgame_line")
    )
}

fn validate_media_request_galgame_target_line(
    media_asset_id: i64,
    asset: &media_asset::Model,
    target_line_id: i64,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
    galgame_lines: &HashMap<i64, &galgame_line::Model>,
) -> Result<(), String> {
    let line = galgame_lines.get(&target_line_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json.target_line_id 引用的 Galgame 行不存在: media_asset_id={}, target_line_id={target_line_id}",
            media_asset_id
        )
    })?;
    let scene = galgame_scenes.get(&line.scene_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json.target_line_id 所属场不存在: media_asset_id={}, target_line_id={target_line_id}, scene_id={}",
            media_asset_id, line.scene_id
        )
    })?;
    validate_media_request_galgame_scene_project(media_asset_id, asset, scene, galgame_scripts)
}

fn validate_optional_media_request_id_ref(
    request: &serde_json::Map<String, Value>,
    key: &str,
    valid_ids: &HashSet<i64>,
    label: &str,
    media_asset_id: i64,
) -> Result<(), String> {
    let Some(value) = optional_media_request_i64(request, key, media_asset_id)? else {
        return Ok(());
    };
    ensure_package!(
        valid_ids.contains(&value),
        "项目族导入包媒体来源 request_snapshot_json.{key} 引用的{label}不存在: media_asset_id={}, {key}={}",
        media_asset_id,
        value
    );
    Ok(())
}

fn optional_media_request_i64(
    request: &serde_json::Map<String, Value>,
    key: &str,
    media_asset_id: i64,
) -> Result<Option<i64>, String> {
    let Some(value) = request.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value.as_i64().map(Some).ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json.{key} 必须是整数: media_asset_id={media_asset_id}"
        )
    })
}

fn required_media_request_i64(
    request: &serde_json::Map<String, Value>,
    key: &str,
    media_asset_id: i64,
) -> Result<i64, String> {
    optional_media_request_i64(request, key, media_asset_id)?.ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json.{key} 缺失: media_asset_id={media_asset_id}"
        )
    })
}

fn required_media_request_string<'a>(
    request: &'a serde_json::Map<String, Value>,
    key: &str,
    media_asset_id: i64,
) -> Result<&'a str, String> {
    let Some(value) = request.get(key) else {
        return Err(format!(
            "项目族导入包媒体来源 request_snapshot_json.{key} 缺失: media_asset_id={media_asset_id}"
        ));
    };
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "项目族导入包媒体来源 request_snapshot_json.{key} 必须是非空字符串: media_asset_id={media_asset_id}"
            )
        })
}

fn validate_media_request_target_entity(
    media_asset_id: i64,
    asset: &media_asset::Model,
    target_entity_kind: &str,
    target_entity_id: i64,
    refs: &MediaRequestRefContext<'_>,
) -> Result<(), String> {
    match target_entity_kind {
        "character_profile" => ensure_package!(
            refs.character_profile_ids.contains(&target_entity_id),
            "项目族导入包媒体来源 request_snapshot_json.target_entity_id 引用的角色档案不存在: media_asset_id={}, target_entity_id={}",
            media_asset_id,
            target_entity_id
        ),
        "segment" => ensure_package!(
            refs.video_segment_ids.contains(&target_entity_id),
            "项目族导入包媒体来源 request_snapshot_json.target_entity_id 引用的视频片段不存在: media_asset_id={}, target_entity_id={}",
            media_asset_id,
            target_entity_id
        ),
        "assembly" => ensure_package!(
            refs.assembly_timeline_ids.contains(&target_entity_id),
            "项目族导入包媒体来源 request_snapshot_json.target_entity_id 引用的成片时间线不存在: media_asset_id={}, target_entity_id={}",
            media_asset_id,
            target_entity_id
        ),
        "galgame_scene" => {
            let scene = refs
                .galgame_scenes
                .get(&target_entity_id)
                .copied()
                .ok_or_else(|| {
                format!(
                    "项目族导入包媒体来源 request_snapshot_json.target_entity_id 引用的 Galgame 场不存在: media_asset_id={}, target_entity_id={target_entity_id}",
                    media_asset_id
                )
            })?;
            validate_media_request_galgame_scene_project(
                media_asset_id,
                asset,
                scene,
                refs.galgame_scripts,
            )?;
        }
        "galgame_line" => {
            let line = refs
                .galgame_lines
                .get(&target_entity_id)
                .copied()
                .ok_or_else(|| {
                format!(
                    "项目族导入包媒体来源 request_snapshot_json.target_entity_id 引用的 Galgame 行不存在: media_asset_id={}, target_entity_id={target_entity_id}",
                    media_asset_id
                )
            })?;
            let scene = refs
                .galgame_scenes
                .get(&line.scene_id)
                .copied()
                .ok_or_else(|| {
                format!(
                    "项目族导入包媒体来源 request_snapshot_json.target_entity_id 行所属场不存在: media_asset_id={}, target_entity_id={}, scene_id={}",
                    media_asset_id, target_entity_id, line.scene_id
                )
            })?;
            validate_media_request_galgame_scene_project(
                media_asset_id,
                asset,
                scene,
                refs.galgame_scripts,
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_media_request_galgame_scene_project(
    media_asset_id: i64,
    asset: &media_asset::Model,
    scene: &galgame_scene::Model,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
) -> Result<(), String> {
    let script = galgame_scripts.get(&scene.script_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json Galgame 场引用了不存在的剧本: media_asset_id={}, scene_id={}, script_id={}",
            media_asset_id, scene.id, scene.script_id
        )
    })?;
    if asset.project_id != script.project_id || asset.root_project_id != script.root_project_id {
        return Err(format!(
            "项目族导入包媒体来源 request_snapshot_json Galgame 目标与媒体资产项目不一致: media_asset_id={}, asset_project_id={}, target_project_id={}",
            media_asset_id, asset.project_id, script.project_id
        ));
    }
    Ok(())
}

fn validate_media_binding_target_ref(
    binding: &media_binding::Model,
    chapter_ids: &HashSet<i64>,
    character_profile_ids: &HashSet<i64>,
    object_profile_ids: &HashSet<i64>,
    scene_profile_ids: &HashSet<i64>,
    video_segment_ids: &HashSet<i64>,
    assembly_timeline_ids: &HashSet<i64>,
    galgame_scene_ids: &HashSet<i64>,
    galgame_line_ids: &HashSet<i64>,
) -> Result<(), String> {
    let exists = match binding.target_kind.as_str() {
        "character_profile" => character_profile_ids.contains(&binding.target_id),
        "object_profile" => object_profile_ids.contains(&binding.target_id),
        "scene_profile" => scene_profile_ids.contains(&binding.target_id),
        "video_segment" => video_segment_ids.contains(&binding.target_id),
        "chapter" => chapter_ids.contains(&binding.target_id),
        "assembly_timeline" => assembly_timeline_ids.contains(&binding.target_id),
        "galgame_scene" => galgame_scene_ids.contains(&binding.target_id),
        "galgame_line" => galgame_line_ids.contains(&binding.target_id),
        _ => return Ok(()),
    };
    ensure_package!(
        exists,
        "项目族导入包媒体绑定目标不存在: media_binding_id={}, target_kind={}, target_id={}",
        binding.id,
        binding.target_kind,
        binding.target_id
    );
    Ok(())
}

fn validate_unique_ids(ids: impl Iterator<Item = i64>, label: &str) -> Result<(), String> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(format!("导入包包含重复 {label}: {id}"));
        }
    }
    Ok(())
}

pub(super) fn validate_package_entry_path(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("zip 媒体条目路径不能为空".to_string());
    }
    if path.contains('\\') {
        return Err(format!("zip 媒体条目路径不能包含反斜杠: {path}"));
    }
    let path_obj = Path::new(path);
    if path_obj.is_absolute() {
        return Err(format!("zip 媒体条目路径不能是绝对路径: {path}"));
    }
    for component in path_obj.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(format!("zip 媒体条目路径非法: {path}"));
        }
    }
    Ok(())
}

pub(super) fn validate_media_bytes(asset: &media_asset::Model, bytes: &[u8]) -> Result<(), String> {
    let expected_size = asset
        .file_size
        .ok_or_else(|| format!("媒体资产缺少 file_size: {}", asset.id))?;
    if expected_size != bytes.len() as i64 {
        return Err(format!(
            "媒体文件大小与清单不一致: media_asset_id={}, expected={}, actual={}",
            asset.id,
            expected_size,
            bytes.len()
        ));
    }
    let expected_checksum = asset
        .checksum
        .as_deref()
        .ok_or_else(|| format!("媒体资产缺少 checksum: {}", asset.id))?;
    let actual_checksum = sha256_bytes(bytes);
    if expected_checksum != actual_checksum {
        return Err(format!(
            "媒体文件 checksum 与清单不一致: media_asset_id={}",
            asset.id
        ));
    }
    Ok(())
}
