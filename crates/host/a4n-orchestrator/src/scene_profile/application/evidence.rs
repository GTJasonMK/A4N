use super::types::SceneProfileSyncRequestContext;
use a4n_domain::source_spans::anchor_quotes_in_exact_chapter;

pub(super) fn resolve_scene_text_anchor_texts(
    path: &str,
    anchor_ids: &[i64],
    context: &SceneProfileSyncRequestContext,
) -> Result<Vec<String>, String> {
    anchor_quotes_in_exact_chapter(
        path,
        anchor_ids,
        &context.text_anchors,
        context.chapter.id,
        "当前章节",
    )
}
