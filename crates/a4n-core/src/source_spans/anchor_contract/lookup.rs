use crate::models::project::text_anchor;

pub(super) fn anchor_by_id<'a>(
    path: &str,
    anchor_id: i64,
    text_anchors: &'a [text_anchor::Model],
) -> Result<&'a text_anchor::Model, String> {
    text_anchors
        .iter()
        .find(|anchor| anchor.id == anchor_id)
        .ok_or_else(|| format!("{path} 无效：text_anchor_id 不在本次输入候选中: {anchor_id}"))
}
