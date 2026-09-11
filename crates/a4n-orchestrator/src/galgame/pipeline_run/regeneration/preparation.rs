use a4n_error::AppError;

use crate::galgame::{
    GalgamePipelinePromptChapter, GalgamePipelineSourceSpan, GalgameSceneRegenerationInput,
    GalgameShootingScriptProjection,
};

use super::contracts::PreparedGalgameSceneRegeneration;

pub(super) fn prepare_galgame_scene_regeneration(
    input: &GalgameSceneRegenerationInput,
) -> Result<PreparedGalgameSceneRegeneration, AppError> {
    if input.source_spans.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "Galgame 场 {} 局部重跑缺少目标源锚点",
            input.scene_id
        )));
    }
    let prompt_chapter = GalgamePipelinePromptChapter {
        source_snapshot_chapter_id: input.source_snapshot_chapter_id,
        source_chapter_id: input.source_chapter_id,
        chapter_number: input.chapter_number,
        title: input.chapter_title.clone(),
        summary: input.chapter_summary.clone(),
        content_hash: input.chapter_content_hash.clone(),
        content: input.chapter_content.clone(),
        source_spans: input.source_spans.clone(),
    };
    let persist_source_spans = input
        .source_spans
        .iter()
        .map(|span| GalgamePipelineSourceSpan {
            id: span.id.clone(),
            text_anchor_id: span.text_anchor_id,
            source_snapshot_chapter_id: span.source_snapshot_chapter_id,
        })
        .collect::<Vec<_>>();
    let mut directing_scene = input.directing_scene.clone();
    directing_scene.index = 1;
    let shooting_script_projection = GalgameShootingScriptProjection {
        scenes: vec![directing_scene],
    };
    let scenes_json =
        serde_json::to_value(&shooting_script_projection).map_err(AppError::Serialization)?;
    Ok(PreparedGalgameSceneRegeneration {
        prompt_chapter,
        persist_source_spans,
        shooting_script_projection,
        scenes_json,
    })
}
