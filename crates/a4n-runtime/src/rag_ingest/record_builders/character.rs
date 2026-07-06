#[cfg(test)]
use super::format_character_record;
use super::{
    DimensionRagSection, dimension_fields_section, dimension_text_section, optional_json_text,
    optional_rag_text, required_rag_text, short_rag_title,
};
use crate::rag_ingest::RagRecord;
use a4n_core::models::character::{
    attribute_event, character_state_index, foreshadowing_index, protagonist_profile,
};
use a4n_core::models::project::blueprint_character;
use a4n_core::rag::RagDataType;

#[cfg(test)]
pub(in crate::rag_ingest) fn protagonist_rag_text(
    profile: &protagonist_profile::Model,
) -> Result<String, String> {
    let name = required_rag_text(
        Some(&profile.name),
        format!("主角档案 {} 缺少姓名，跳过 RAG 入库", profile.id),
    )?;
    let mut lines = vec![format_character_record(
        name,
        &[
            ("性别", profile.gender.as_deref()),
            ("年龄", profile.age.as_deref()),
            ("外貌", profile.appearance.as_deref()),
            ("背景", profile.background.as_deref()),
            ("人格", profile.personality.as_deref()),
        ],
    )];
    if let Some(base_attributes) = optional_json_text(Some(&profile.base_attributes)) {
        lines.push(format!("基础属性: {base_attributes}"));
    }
    if let Some(current_state) = optional_json_text(Some(&profile.current_state)) {
        lines.push(format!("当前状态: {current_state}"));
    }
    if let Some(character_arc) = optional_json_text(profile.character_arc.as_ref()) {
        lines.push(format!("角色弧光: {character_arc}"));
    }
    if let Some(voice_fingerprint) = optional_json_text(profile.voice_fingerprint.as_ref()) {
        lines.push(format!("声音指纹: {voice_fingerprint}"));
    }
    Ok(lines.join("\n"))
}

pub(in crate::rag_ingest) fn protagonist_rag_sections(
    profile: &protagonist_profile::Model,
) -> Result<Vec<DimensionRagSection>, String> {
    let name = required_rag_text(
        Some(&profile.name),
        format!("主角档案 {} 缺少姓名，跳过 RAG 入库", profile.id),
    )?;
    let mut sections = Vec::new();
    if let Some(section) = dimension_fields_section(
        "主角档案",
        "identity",
        "身份",
        name,
        &[
            ("性别", profile.gender.as_deref()),
            ("年龄", profile.age.as_deref()),
        ],
        true,
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_fields_section(
        "主角档案",
        "appearance",
        "外貌",
        name,
        &[("外貌", profile.appearance.as_deref())],
        false,
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_fields_section(
        "主角档案",
        "background",
        "背景",
        name,
        &[("背景", profile.background.as_deref())],
        false,
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_fields_section(
        "主角档案",
        "personality",
        "人格",
        name,
        &[("人格", profile.personality.as_deref())],
        false,
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_text_section(
        "主角档案",
        "base_attributes",
        "基础属性",
        name,
        "基础属性",
        optional_json_text(Some(&profile.base_attributes)),
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_text_section(
        "主角档案",
        "current_state",
        "当前状态",
        name,
        "当前状态",
        optional_json_text(Some(&profile.current_state)),
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_text_section(
        "主角档案",
        "character_arc",
        "角色弧光",
        name,
        "角色弧光",
        optional_json_text(profile.character_arc.as_ref()),
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_text_section(
        "主角档案",
        "voice_fingerprint",
        "声音指纹",
        name,
        "声音指纹",
        optional_json_text(profile.voice_fingerprint.as_ref()),
    ) {
        sections.push(section);
    }
    if sections.is_empty() {
        return Err(format!(
            "主角档案 {} 没有可入库内容，跳过 RAG 入库",
            profile.id
        ));
    }
    Ok(sections)
}

pub(in crate::rag_ingest) fn protagonist_rag_records(
    project_id: i64,
    profile: &protagonist_profile::Model,
) -> Result<Vec<RagRecord>, String> {
    let name = required_rag_text(
        Some(&profile.name),
        format!("主角档案 {} 缺少姓名，跳过 RAG 入库", profile.id),
    )?;
    let sections = protagonist_rag_sections(profile)?;
    Ok(sections
        .into_iter()
        .enumerate()
        .map(|(section_index, section)| {
            let title = format!("{name} 主角档案: {}", section.label);
            RagRecord {
                chunk_id: format!(
                    "protagonist:{project_id}:{}:{}",
                    profile.id, section.dimension
                ),
                text: section.text,
                metadata: serde_json::json!({
                    "data_type": RagDataType::Protagonist.as_str(),
                    "profile_id": profile.id,
                    "character_name": name,
                    "dimension": section.dimension,
                    "section_index": section_index,
                    "title": title,
                }),
            }
        })
        .collect())
}

#[cfg(test)]
pub(in crate::rag_ingest) fn blueprint_character_rag_text(
    character: &blueprint_character::Model,
) -> Result<String, String> {
    let name = required_rag_text(
        Some(&character.name),
        format!("蓝图角色 {} 缺少姓名，跳过 RAG 入库", character.id),
    )?;
    let character_type = required_rag_text(
        Some(&character.character_type),
        format!("蓝图角色 {} 缺少类型，跳过 RAG 入库", character.id),
    )?;
    let arc_stages_text = optional_json_text(character.arc_stages.as_ref());
    Ok(format_character_record(
        name,
        &[
            ("类型", Some(character_type)),
            ("性别", character.gender.as_deref()),
            ("年龄", character.age.as_deref()),
            ("外貌", character.appearance.as_deref()),
            ("性格", character.personality.as_deref()),
            ("背景", character.background.as_deref()),
            ("动机", character.motivation.as_deref()),
            ("角色弧光", character.arc_summary.as_deref()),
            (
                "与主角关系",
                character.relationship_to_protagonist.as_deref(),
            ),
            ("内在弧线", arc_stages_text.as_deref()),
        ],
    ))
}

pub(in crate::rag_ingest) fn blueprint_character_rag_sections(
    character: &blueprint_character::Model,
) -> Result<Vec<DimensionRagSection>, String> {
    let name = required_rag_text(
        Some(&character.name),
        format!("蓝图角色 {} 缺少姓名，跳过 RAG 入库", character.id),
    )?;
    let character_type = required_rag_text(
        Some(&character.character_type),
        format!("蓝图角色 {} 缺少类型，跳过 RAG 入库", character.id),
    )?;
    let arc_stages_text = optional_json_text(character.arc_stages.as_ref());
    let mut sections = Vec::new();
    if let Some(section) = dimension_fields_section(
        "角色设定",
        "identity",
        "身份",
        name,
        &[
            ("类型", Some(character_type)),
            ("性别", character.gender.as_deref()),
            ("年龄", character.age.as_deref()),
        ],
        true,
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_fields_section(
        "角色设定",
        "appearance",
        "外貌",
        name,
        &[("外貌", character.appearance.as_deref())],
        false,
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_fields_section(
        "角色设定",
        "personality",
        "性格",
        name,
        &[("性格", character.personality.as_deref())],
        false,
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_fields_section(
        "角色设定",
        "background",
        "背景",
        name,
        &[("背景", character.background.as_deref())],
        false,
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_fields_section(
        "角色设定",
        "motivation",
        "动机",
        name,
        &[("动机", character.motivation.as_deref())],
        false,
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_fields_section(
        "角色设定",
        "arc",
        "角色弧光",
        name,
        &[
            ("角色弧光", character.arc_summary.as_deref()),
            ("内在弧线", arc_stages_text.as_deref()),
        ],
        false,
    ) {
        sections.push(section);
    }
    if let Some(section) = dimension_fields_section(
        "角色设定",
        "relationship_to_protagonist",
        "与主角关系",
        name,
        &[(
            "与主角关系",
            character.relationship_to_protagonist.as_deref(),
        )],
        false,
    ) {
        sections.push(section);
    }
    if sections.is_empty() {
        return Err(format!(
            "蓝图角色 {} 没有可入库内容，跳过 RAG 入库",
            character.id
        ));
    }
    Ok(sections)
}

pub(in crate::rag_ingest) fn blueprint_character_rag_records(
    project_id: i64,
    character: &blueprint_character::Model,
) -> Result<Vec<RagRecord>, String> {
    let name = required_rag_text(
        Some(&character.name),
        format!("蓝图角色 {} 缺少姓名，跳过 RAG 入库", character.id),
    )?;
    let sections = blueprint_character_rag_sections(character)?;
    Ok(sections
        .into_iter()
        .enumerate()
        .map(|(section_index, section)| {
            let title = format!("{name} 角色设定: {}", section.label);
            RagRecord {
                chunk_id: format!(
                    "character:{project_id}:blueprint:{}:{}",
                    character.id, section.dimension
                ),
                text: section.text,
                metadata: serde_json::json!({
                    "data_type": RagDataType::Character.as_str(),
                    "source": "blueprint_character",
                    "character_id": character.id,
                    "character_name": name,
                    "dimension": section.dimension,
                    "section_index": section_index,
                    "title": title,
                }),
            }
        })
        .collect())
}

pub(in crate::rag_ingest) fn character_state_rag_text(
    state_index: &character_state_index::Model,
    name: &str,
) -> Result<String, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色状态 {} 缺少角色名，跳过 RAG 入库", state_index.id),
    )?;
    let state_summary = required_rag_text(
        Some(&state_index.state_summary),
        format!("角色状态 {} 缺少状态摘要，跳过 RAG 入库", state_index.id),
    )?;
    let mut lines = vec![
        format!("角色: {name}"),
        format!("章节: 第{}章", state_index.chapter_number),
        format!("状态: {state_summary}"),
    ];
    if let Some(emotional_state) = optional_rag_text(state_index.emotional_state.as_deref()) {
        lines.push(format!("情绪: {emotional_state}"));
    }
    if let Some(location) = optional_rag_text(state_index.current_location.as_deref()) {
        lines.push(format!("位置: {location}"));
    }
    if let Some(goal) = optional_rag_text(state_index.current_goal.as_deref()) {
        lines.push(format!("目标: {goal}"));
    }
    if let Some(attributes) = optional_json_text(Some(&state_index.key_attributes)) {
        lines.push(format!("关键属性: {attributes}"));
    }
    Ok(lines.join("\n"))
}

pub(crate) fn character_state_rag_record(
    project_id: i64,
    state_index: &character_state_index::Model,
    name: &str,
) -> Result<RagRecord, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色状态 {} 缺少角色名，跳过 RAG 入库", state_index.id),
    )?;
    let text = character_state_rag_text(state_index, name)?;
    let title = format!("{name} 第{}章后状态", state_index.chapter_number);
    Ok(RagRecord {
        chunk_id: format!("character_state:{project_id}:{}", state_index.id),
        text,
        metadata: serde_json::json!({
            "data_type": RagDataType::CharacterState.as_str(),
            "state_id": state_index.id,
            "profile_id": state_index.profile_id,
            "character_name": name,
            "chapter_number": state_index.chapter_number,
            "title": title,
        }),
    })
}

fn character_attribute_event_rag_text(
    event: &attribute_event::Model,
    name: &str,
) -> Result<String, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色属性事件 {} 缺少角色名，跳过 RAG 入库", event.id),
    )?;
    let category = required_rag_text(
        Some(&event.category),
        format!("角色属性事件 {} 缺少属性类别，跳过 RAG 入库", event.id),
    )?;
    let key = required_rag_text(
        Some(&event.key),
        format!("角色属性事件 {} 缺少属性键，跳过 RAG 入库", event.id),
    )?;
    let operation = required_rag_text(
        Some(&event.operation),
        format!("角色属性事件 {} 缺少操作类型，跳过 RAG 入库", event.id),
    )?;

    let mut lines = vec![
        format!("角色: {name}"),
        format!("章节: 第{}章", event.chapter_number),
        format!("类别: {}", attribute_category_label(category)),
        format!("属性: {key}"),
        format!("操作: {}", attribute_operation_label(operation)),
    ];
    if let Some(old_value) = optional_json_text(Some(&event.old_value)) {
        lines.push(format!("原值: {old_value}"));
    }
    if let Some(new_value) = optional_json_text(Some(&event.new_value)) {
        lines.push(format!("新值: {new_value}"));
    }
    if let Some(description) = optional_rag_text(Some(&event.change_description)) {
        lines.push(format!("摘要: {description}"));
    }
    if let Some(cause) = optional_rag_text(Some(&event.cause)) {
        lines.push(format!("原因: {cause}"));
    }
    if let Some(evidence) = optional_rag_text(Some(&event.evidence)) {
        lines.push(format!("证据: {evidence}"));
    }
    Ok(lines.join("\n"))
}

pub(crate) fn character_attribute_event_rag_record(
    project_id: i64,
    event: &attribute_event::Model,
    name: &str,
) -> Result<RagRecord, String> {
    let name = required_rag_text(
        Some(name),
        format!("角色属性事件 {} 缺少角色名，跳过 RAG 入库", event.id),
    )?;
    let text = character_attribute_event_rag_text(event, name)?;
    let title = format!("{name} 第{}章属性变更", event.chapter_number);
    Ok(RagRecord {
        chunk_id: format!("character_attribute_event:{project_id}:{}", event.id),
        text,
        metadata: serde_json::json!({
            "data_type": RagDataType::CharacterAttributeEvent.as_str(),
            "attribute_event_id": event.id,
            "profile_id": event.profile_id,
            "character_name": name,
            "chapter_number": event.chapter_number,
            "category": event.category,
            "attribute_key": event.key,
            "operation": event.operation,
            "title": title,
        }),
    })
}

pub(in crate::rag_ingest) fn foreshadowing_rag_text(
    item: &foreshadowing_index::Model,
) -> Result<String, String> {
    let description = required_rag_text(
        Some(&item.description),
        format!("伏笔 {} 缺少描述，跳过 RAG 入库", item.id),
    )?;
    let status = required_rag_text(
        Some(&item.status),
        format!("伏笔 {} 缺少状态，跳过 RAG 入库", item.id),
    )?;

    let mut lines = vec![
        format!("伏笔: {description}"),
        format!("埋设章节: 第{}章", item.setup_chapter),
        format!("状态: {status}"),
        format!("重要度: {}", item.importance),
    ];
    if let Some(payoff_chapter) = item.payoff_chapter {
        lines.push(format!("揭示章节: 第{payoff_chapter}章"));
    }
    if let Some(related_characters) = optional_json_text(item.related_characters.as_ref()) {
        lines.push(format!("关联角色/实体: {related_characters}"));
    }
    Ok(lines.join("\n"))
}

pub(crate) fn foreshadowing_rag_record(
    project_id: i64,
    item: &foreshadowing_index::Model,
) -> Result<RagRecord, String> {
    let description = required_rag_text(
        Some(&item.description),
        format!("伏笔 {} 缺少描述，跳过 RAG 入库", item.id),
    )?;
    let text = foreshadowing_rag_text(item)?;
    let title = format!(
        "伏笔 第{}章: {}",
        item.setup_chapter,
        short_rag_title(description, 48)
    );
    Ok(RagRecord {
        chunk_id: format!("foreshadowing:{project_id}:{}", item.id),
        text,
        metadata: serde_json::json!({
            "data_type": RagDataType::Foreshadowing.as_str(),
            "foreshadowing_id": item.id,
            "chapter_number": item.setup_chapter,
            "setup_chapter": item.setup_chapter,
            "payoff_chapter": item.payoff_chapter,
            "status": item.status,
            "importance": item.importance,
            "related_characters": item.related_characters,
            "title": title,
        }),
    })
}

fn attribute_category_label(category: &str) -> &'static str {
    match category {
        "explicit" => "显性属性",
        "implicit" => "隐性属性",
        "social" => "社会属性",
        _ => "属性",
    }
}

fn attribute_operation_label(operation: &str) -> &'static str {
    match operation {
        "add" => "新增",
        "modify" => "修改",
        "delete" => "删除",
        _ => "变更",
    }
}

#[cfg(test)]
pub(in crate::rag_ingest) fn expected_blueprint_character_record_count(
    project_id: i64,
    character: &blueprint_character::Model,
) -> usize {
    blueprint_character_rag_records(project_id, character)
        .map(|records| records.len())
        .unwrap_or(0)
}

#[cfg(test)]
pub(in crate::rag_ingest) fn expected_protagonist_record_count(
    project_id: i64,
    profile: &protagonist_profile::Model,
) -> usize {
    protagonist_rag_records(project_id, profile)
        .map(|records| records.len())
        .unwrap_or(0)
}
