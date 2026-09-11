use super::formatting::{
    format_blueprint_core_section, format_character_reference_line,
    format_foreshadowing_reference_line, format_protagonist_profile_reference,
    format_relationship_reference_line,
};
use super::json::{is_meaningful_json_value, truncate_text_chars};
use super::selection::{
    outline_character_names, select_generation_characters, select_generation_foreshadowing,
    select_generation_relationships,
};
use crate::OrchestratorContext;
use a4n_db_models::models::chapter::chapter_outline;

impl OrchestratorContext {
    pub async fn resolve_generation_reference_context(
        &self,
        project_id: i64,
        chapter_number: i32,
        outline: Option<&chapter_outline::Model>,
    ) -> Result<Option<String>, String> {
        self.build_generation_reference_context(project_id, chapter_number, outline)
            .await
            .map_err(|err| format!("构建章节生成关键参考失败: {err}"))
    }

    async fn build_generation_reference_context(
        &self,
        project_id: i64,
        chapter_number: i32,
        outline: Option<&chapter_outline::Model>,
    ) -> Result<Option<String>, String> {
        let (rows_result, foreshadowing_result) = futures::join!(
            self.chapter_generation_reference_context_rows(project_id),
            self.pending_foreshadowing_for_project(project_id, 20),
        );
        let rows = rows_result.map_err(|error| error.to_string())?;
        let blueprint = rows.blueprint;
        let characters = rows.characters;
        let relationships = rows.relationships;
        let protagonist_profiles = rows.protagonist_profiles;
        let foreshadowing = foreshadowing_result.map_err(|error| error.to_string())?;

        let mut sections = Vec::new();
        if let Some(blueprint) = blueprint.as_ref() {
            sections.push(format_blueprint_core_section(blueprint, &characters));
        } else if !characters.is_empty() {
            let names = characters
                .iter()
                .map(|character| character.name.as_str())
                .collect::<Vec<_>>();
            sections.push(format!("## 核心设定\n角色名单: {}", names.join("、")));
        }

        let involved_names = outline_character_names(outline);
        let involved_characters = select_generation_characters(&characters, &involved_names);
        let relevant_relationships =
            select_generation_relationships(&relationships, &involved_names);
        let pending_foreshadowing =
            select_generation_foreshadowing(&foreshadowing, chapter_number, &involved_names);

        let mut reference_lines = Vec::new();
        if !involved_characters.is_empty() {
            reference_lines.push("### 涉及角色".to_string());
            reference_lines.extend(
                involved_characters
                    .iter()
                    .map(format_character_reference_line),
            );
        }
        if !relevant_relationships.is_empty() {
            if !reference_lines.is_empty() {
                reference_lines.push(String::new());
            }
            reference_lines.push("### 角色关系".to_string());
            reference_lines.extend(
                relevant_relationships
                    .iter()
                    .map(format_relationship_reference_line),
            );
        }
        if let Some(profile_section) =
            format_protagonist_profile_reference(&protagonist_profiles, &involved_names)
        {
            if !reference_lines.is_empty() {
                reference_lines.push(String::new());
            }
            reference_lines.extend(profile_section);
        }
        if !pending_foreshadowing.is_empty() {
            if !reference_lines.is_empty() {
                reference_lines.push(String::new());
            }
            reference_lines.push("### 待回收伏笔".to_string());
            reference_lines.extend(
                pending_foreshadowing
                    .iter()
                    .map(format_foreshadowing_reference_line),
            );
        }
        if !reference_lines.is_empty() {
            sections.push(format!("## 关键参考\n{}", reference_lines.join("\n")));
        }

        if let Some(world_setting) = blueprint
            .as_ref()
            .and_then(|blueprint| blueprint.world_setting.as_ref())
            .filter(|value| is_meaningful_json_value(value))
        {
            let text = serde_json::to_string_pretty(world_setting)
                .map_err(|error| format!("序列化章节生成世界观设定失败: {error}"))?;
            sections.push(format!(
                "## 补充参考\n世界观设定:\n{}",
                truncate_text_chars(&text, 900)
            ));
        }

        if sections.is_empty() {
            Ok(None)
        } else {
            Ok(Some(sections.join("\n\n")))
        }
    }
}
