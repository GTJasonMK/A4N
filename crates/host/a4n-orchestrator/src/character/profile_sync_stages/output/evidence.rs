use serde_json::Value;
use std::collections::HashMap;

use super::fields::required_positive_i64;
use crate::character::{required_array, required_i32_at};

pub(super) struct ChapterEvidenceIndex<'a> {
    chapters: HashMap<i32, HashMap<i64, &'a str>>,
}

impl<'a> ChapterEvidenceIndex<'a> {
    pub(super) fn new(chapter_data: &'a [Value]) -> Result<Self, String> {
        if chapter_data.is_empty() {
            return Err("角色同步缺少可核对证据的章节正文".into());
        }
        let mut chapters = HashMap::with_capacity(chapter_data.len());
        for (index, chapter) in chapter_data.iter().enumerate() {
            let chapter_number = required_i32_at(
                chapter,
                "chapter_number",
                &format!("chapters[{index}].chapter_number"),
            )?;
            let mut by_id = HashMap::new();
            for (anchor_index, anchor) in
                required_array(chapter, "text_anchors")?.iter().enumerate()
            {
                let path = format!("chapters[{index}].text_anchors[{anchor_index}]");
                let anchor_id = required_positive_i64(anchor, "id", &format!("{path}.id"))?;
                let quote = anchor
                    .get("quote")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|quote| !quote.is_empty())
                    .ok_or_else(|| format!("{path}.quote 必须是非空字符串"))?;
                if by_id.insert(anchor_id, quote).is_some() {
                    return Err(format!(
                        "第 {chapter_number} 章 text_anchors 包含重复 id: {anchor_id}"
                    ));
                }
            }
            if chapters.insert(chapter_number, by_id).is_some() {
                return Err(format!("角色同步证据包含重复章节: 第 {chapter_number} 章"));
            }
        }
        Ok(Self { chapters })
    }

    pub(super) fn quote(
        &self,
        item: &Value,
        path: &str,
        anchor_field: &str,
    ) -> Result<&'a str, String> {
        let chapter_number =
            required_i32_at(item, "chapter_number", &format!("{path}.chapter_number"))?;
        let anchor_id =
            required_positive_i64(item, anchor_field, &format!("{path}.{anchor_field}"))?;
        let anchors = self
            .chapters
            .get(&chapter_number)
            .ok_or_else(|| format!("{path} 缺少可核对证据的第 {chapter_number} 章正文"))?;
        anchors.get(&anchor_id).copied().ok_or_else(|| {
            format!("{path}.{anchor_field} 不在第 {chapter_number} 章 text_anchors 中：{anchor_id}")
        })
    }

    pub(super) fn materialize(
        &self,
        items: &mut [Value],
        label: &str,
        anchor_field: &str,
        target_field: &str,
    ) -> Result<(), String> {
        for (index, item) in items.iter_mut().enumerate() {
            let path = format!("{label}[{index}]");
            let text = self.quote(item, &path, anchor_field)?.to_string();
            let object = item
                .as_object_mut()
                .ok_or_else(|| format!("{path} 必须是 JSON 对象"))?;
            object.insert(target_field.to_string(), Value::String(text));
            object.remove(anchor_field);
        }
        Ok(())
    }
}
