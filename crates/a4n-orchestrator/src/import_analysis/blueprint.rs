use a4n_core::models::{chapter::chapter, project::novel_project};
use serde_json::json;

use super::chapter_outline::{collect_analysis_strings, collect_chapter_summaries};
use crate::{OrchestratorContext, project::ImportBlueprintUpsertInput};

fn import_blueprint_genre(project_type: &str) -> Result<&'static str, String> {
    match project_type.trim() {
        "novel_long" => Ok("长篇小说"),
        "novel_short" => Ok("短篇小说"),
        "script" => Ok("剧本"),
        "prose" => Ok("散文"),
        "" => Err("项目类型不能为空，无法生成导入蓝图题材".to_string()),
        other => Err(format!("不支持的项目类型：{other}，无法生成导入蓝图题材")),
    }
}

pub async fn upsert_import_blueprint(
    context: &OrchestratorContext,
    project: &novel_project::Model,
    chapters: &[chapter::Model],
    needs_part_outlines: bool,
) -> Result<(), String> {
    let summaries = collect_chapter_summaries(chapters, 80)?;
    if summaries.is_empty() {
        return Err("项目没有可用于生成导入蓝图的章节摘要".to_string());
    }
    let synopsis = summaries.join("\n");
    let world_setting = json!({
        "locations": collect_analysis_strings(chapters, "locations", 40),
        "items": collect_analysis_strings(chapters, "items", 40),
        "tags": collect_analysis_strings(chapters, "tags", 40),
        "source": "import_analysis"});
    let one_sentence = summaries
        .first()
        .cloned()
        .ok_or_else(|| "项目没有可用于生成一句话概要的章节摘要".to_string())?;
    let chapters_per_part = if needs_part_outlines { Some(25) } else { None };
    let total_chapters =
        i32::try_from(chapters.len()).map_err(|_| "导入蓝图章节数量超出 i32 范围".to_string())?;

    context
        .upsert_import_blueprint(ImportBlueprintUpsertInput {
            project_id: project.id,
            title: project.title.clone(),
            genre: import_blueprint_genre(&project.project_type)?.to_string(),
            one_sentence_summary: one_sentence,
            full_synopsis: synopsis,
            world_setting,
            total_chapters,
            needs_part_outlines,
            chapters_per_part,
        })
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::import_blueprint_genre;

    #[test]
    fn import_blueprint_genre_maps_known_project_types() {
        assert_eq!(import_blueprint_genre("novel_long").unwrap(), "长篇小说");
        assert_eq!(import_blueprint_genre("novel_short").unwrap(), "短篇小说");
        assert_eq!(import_blueprint_genre("script").unwrap(), "剧本");
        assert_eq!(import_blueprint_genre("prose").unwrap(), "散文");
    }

    #[test]
    fn import_blueprint_genre_rejects_unknown_project_type() {
        let err = import_blueprint_genre("game")
            .expect_err("unknown project type must not become placeholder genre");

        assert!(err.contains("不支持的项目类型"));
    }
}
