use super::*;

pub(super) fn attribute_counts(value: &Value) -> Result<AttributeCountsSummary, WorkerFailure> {
    let count = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_object)
            .map(|attributes| attributes.len())
            .ok_or_else(|| WorkerFailure::invalid_request(format!("角色当前状态缺少字段 {key}")))
    };
    Ok(AttributeCountsSummary {
        explicit: count("explicit_attributes")?,
        implicit: count("implicit_attributes")?,
        social: count("social_attributes")?,
    })
}

pub(super) fn character_profile_summary(
    profile: &a4n_domain::character::CharacterProfileRecord,
) -> Result<CharacterProfileSummary, WorkerFailure> {
    character_profile_summary_with_state(profile, false, true)
}

pub(super) fn character_profile_summary_with_state(
    profile: &a4n_domain::character::CharacterProfileRecord,
    timeline_started: bool,
    initial_profile_editable: bool,
) -> Result<CharacterProfileSummary, WorkerFailure> {
    Ok(CharacterProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        name: profile.name.clone(),
        gender: profile.gender.clone(),
        age: profile.age.clone(),
        base_attributes: profile.base_attributes.clone(),
        current_state: profile.current_state.clone(),
        character_arc: profile.character_arc.clone(),
        voice_fingerprint: profile.voice_fingerprint.clone(),
        appearance: profile.appearance.clone(),
        background: profile.background.clone(),
        personality: profile.personality.clone(),
        last_synced_chapter: profile.last_synced_chapter,
        timeline_started,
        initial_profile_editable,
        attribute_counts: attribute_counts(&profile.current_state)?,
        created_at: Some(profile.created_at.and_utc().to_rfc3339()),
        updated_at: Some(profile.updated_at.and_utc().to_rfc3339()),
    })
}

pub(super) fn attribute_event_summary(
    event: &a4n_domain::character::CharacterAttributeEventRecord,
) -> Result<AttributeEventSummary, WorkerFailure> {
    let source_kind = CharacterTimelineSourceKind::parse(&event.source_kind).ok_or_else(|| {
        WorkerFailure::conflict(format!(
            "角色时间线 source_kind 无效: {}",
            event.source_kind
        ))
    })?;
    let category = CharacterAttributeCategory::parse(&event.category)
        .ok_or_else(|| WorkerFailure::conflict(format!("角色属性分类无效: {}", event.category)))?;
    let operation = CharacterAttributeOperation::parse(&event.operation)
        .ok_or_else(|| WorkerFailure::conflict(format!("角色属性操作无效: {}", event.operation)))?;
    Ok(AttributeEventSummary {
        id: event.id,
        chapter_number: event.chapter_number,
        source_kind,
        timestamp: Some(event.timestamp.and_utc().to_rfc3339()),
        category,
        key: event.key.clone(),
        operation,
        change_description: event.change_description.clone(),
        old_value: event.old_value.clone(),
        new_value: event.new_value.clone(),
        cause: event.cause.clone(),
        evidence: event.evidence.clone(),
    })
}

pub(super) fn behavior_record_summary(
    record: &a4n_domain::character::CharacterBehaviorRecord,
) -> Result<BehaviorRecordSummary, WorkerFailure> {
    let source_kind = CharacterTimelineSourceKind::parse(&record.source_kind).ok_or_else(|| {
        WorkerFailure::conflict(format!(
            "角色时间线 source_kind 无效: {}",
            record.source_kind
        ))
    })?;
    Ok(BehaviorRecordSummary {
        id: record.id,
        chapter_number: record.chapter_number,
        source_kind,
        behavior_description: record.behavior_description.clone(),
        behavior_type: record.behavior_type.clone(),
        original_text: record.original_text.clone(),
        behavior_tags: record.behavior_tags.clone(),
        classification_results: record.classification_results.clone(),
        related_characters: record.related_characters.clone(),
        impact_analysis: record.impact_analysis.clone(),
    })
}

pub(super) fn deletion_mark_summary(
    mark: &a4n_domain::character::CharacterDeletionMarkRecord,
) -> Result<DeletionMarkSummary, WorkerFailure> {
    let source_kind = CharacterTimelineSourceKind::parse(&mark.source_kind).ok_or_else(|| {
        WorkerFailure::conflict(format!("角色时间线 source_kind 无效: {}", mark.source_kind))
    })?;
    let category = CharacterAttributeCategory::parse(&mark.category)
        .ok_or_else(|| WorkerFailure::conflict(format!("角色属性分类无效: {}", mark.category)))?;
    Ok(DeletionMarkSummary {
        id: mark.id,
        source_kind,
        category,
        key: mark.key.clone(),
        reason: mark.reason.clone(),
        evidence: mark.evidence.clone(),
        consecutive_count: mark.consecutive_count,
        last_marked_chapter: mark.last_marked_chapter,
        is_executed: mark.is_executed,
        created_at: mark.created_at.and_utc().to_rfc3339(),
        updated_at: mark.updated_at,
    })
}

fn snapshot_data(
    snapshot: &a4n_domain::character::CharacterSnapshotReferenceRecord,
) -> Result<
    (
        Value,
        AttributeCountsSummary,
        Option<String>,
        Option<String>,
        usize,
        usize,
    ),
    WorkerFailure,
> {
    let attributes =
        a4n_db::character::snapshot_attributes_from_data(&snapshot.data, snapshot.chapter_number)
            .map_err(character_db_error)?;
    let counts = attribute_counts(&attributes)?;
    let label = format!("第 {} 章角色快照", snapshot.chapter_number);
    let count = |name: &str| {
        let value = snapshot
            .data
            .get(name)
            .and_then(Value::as_u64)
            .ok_or_else(|| WorkerFailure::conflict(format!("{label}字段 {name} 必须是非负整数")))?;
        usize::try_from(value)
            .map_err(|_| WorkerFailure::conflict(format!("{label}字段 {name} 超出 usize 范围")))
    };
    Ok((
        attributes,
        counts,
        snapshot
            .data
            .get("state_summary")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        snapshot
            .data
            .get("current_location")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        count("changes_in_chapter")?,
        count("behaviors_in_chapter")?,
    ))
}

pub(super) fn snapshot_reference_summary(
    snapshot: &a4n_domain::character::CharacterSnapshotReferenceRecord,
) -> Result<SnapshotReferenceSummary, WorkerFailure> {
    let (_, counts, state_summary, _, changes, behaviors) = snapshot_data(snapshot)?;
    Ok(SnapshotReferenceSummary {
        id: snapshot.id,
        chapter_number: snapshot.chapter_number,
        snapshot_hash: snapshot.snapshot_hash.clone(),
        attribute_counts: counts,
        state_summary,
        changes_in_chapter: changes,
        behaviors_in_chapter: behaviors,
    })
}

pub(super) fn profile_state_summary(
    snapshot: &a4n_domain::character::CharacterSnapshotReferenceRecord,
) -> Result<ProfileChapterStateSummary, WorkerFailure> {
    let (attributes, counts, state_summary, current_location, changes, behaviors) =
        snapshot_data(snapshot)?;
    let category = |name: &str| {
        attributes
            .get(name)
            .and_then(Value::as_object)
            .map(|object| Value::Object(object.clone()))
            .ok_or_else(|| WorkerFailure::conflict(format!("角色快照属性缺少分类 {name}")))
    };
    Ok(ProfileChapterStateSummary {
        chapter_number: snapshot.chapter_number,
        attribute_counts: counts,
        explicit_attributes: category("explicit_attributes")?,
        implicit_attributes: category("implicit_attributes")?,
        social_attributes: category("social_attributes")?,
        state_summary,
        current_location,
        changes_in_chapter: changes,
        behaviors_in_chapter: behaviors,
        created_at: Some(snapshot.created_at.and_utc().to_rfc3339()),
    })
}
