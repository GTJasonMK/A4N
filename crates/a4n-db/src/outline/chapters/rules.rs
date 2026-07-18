use std::collections::BTreeMap;

use a4n_core::models::chapter::chapter_outline;

use super::super::types::OutlineDbError;

pub(super) fn index_chapter_outlines_by_number(
    outlines: Vec<chapter_outline::Model>,
) -> Result<BTreeMap<i32, chapter_outline::Model>, OutlineDbError> {
    let mut outlines_by_number = BTreeMap::new();
    for outline in outlines {
        if let Some(previous) = outlines_by_number.insert(outline.chapter_number, outline.clone()) {
            return Err(OutlineDbError::InvalidArgument(format!(
                "第 {} 章已有重复章纲记录：{}, {}，请先清理后再应用节拍映射",
                outline.chapter_number, previous.id, outline.id
            )));
        }
    }
    Ok(outlines_by_number)
}
