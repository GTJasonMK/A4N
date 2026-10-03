use std::collections::{HashMap, HashSet};

use a4n_domain::source_spans::{
    SourceSpan, source_marker_text, source_span_marker_excerpt, source_span_marker_text,
};
use a4n_error::AppError;
use serde_json::{Map, Value};

const SOURCE_REFERENCE_FIELDS: &[&str] = &[
    "source_span_id",
    "source_span_ids",
    "allowed_source_span_ids",
    "available_source_span_ids",
    "context_source_span_ids",
    "key_source_span_ids",
    "original_text_span_id",
    "evidence_span_id",
];

const SOURCE_SPAN_RECORD_FIELDS: &[&str] = &["source_spans", "available_source_spans"];
const SOURCE_MARKER_PROTOCOL: &str = "原文保留原有换行作为段落边界；来源标签格式是 ASCII 井号加十进制序号，标签后有一个空格再接原文。来源引用字段只能原样返回本输入中实际出现的短标签；正文其他位置的普通井号不是来源引用。无来源时沿用原合同中的 null 或空数组。";

#[derive(Debug, Clone)]
pub(crate) struct SourceMarkerMap {
    marker_by_source_id: HashMap<String, String>,
    source_id_by_marker: HashMap<String, String>,
}

#[derive(Clone, Copy)]
enum Direction {
    Encode,
    Decode,
}

impl SourceMarkerMap {
    pub(crate) fn from_source_spans(spans: &[SourceSpan]) -> Result<Self, AppError> {
        let mut reserved_markers = HashSet::new();
        for span in spans {
            collect_marker_prefixes(&span.text, &mut reserved_markers);
        }
        Self::from_ids_avoiding(spans.iter().map(|span| span.id.clone()), reserved_markers)
    }

    pub(crate) fn from_spans_and_values<'a>(
        spans: &[SourceSpan],
        values: impl IntoIterator<Item = &'a Value>,
    ) -> Result<Self, AppError> {
        let values = values.into_iter().collect::<Vec<_>>();
        let mut ids = spans.iter().map(|span| span.id.clone()).collect::<Vec<_>>();
        let mut reserved_markers = HashSet::new();
        for span in spans {
            collect_marker_prefixes(&span.text, &mut reserved_markers);
        }
        for value in values {
            collect_reference_ids(value, &mut ids);
            collect_value_marker_prefixes(value, &mut reserved_markers);
        }
        Self::from_ids_avoiding(ids, reserved_markers)
    }

    pub(crate) fn from_ids_and_values<'a>(
        ids: impl IntoIterator<Item = String>,
        values: impl IntoIterator<Item = &'a Value>,
    ) -> Result<Self, AppError> {
        let mut ids = ids.into_iter().collect::<Vec<_>>();
        let values = values.into_iter().collect::<Vec<_>>();
        let mut reserved_markers = HashSet::new();
        for value in values {
            collect_reference_ids(value, &mut ids);
            collect_value_marker_prefixes(value, &mut reserved_markers);
        }
        Self::from_ids_avoiding(ids, reserved_markers)
    }

    #[cfg(test)]
    pub(crate) fn from_ids(ids: impl IntoIterator<Item = String>) -> Result<Self, AppError> {
        Self::from_ids_avoiding(ids, HashSet::new())
    }

    fn from_ids_avoiding(
        ids: impl IntoIterator<Item = String>,
        reserved_markers: HashSet<String>,
    ) -> Result<Self, AppError> {
        let mut marker_by_source_id = HashMap::new();
        let mut source_id_by_marker = HashMap::new();
        let mut seen = HashSet::new();
        let mut marker_index = 0;
        for id in ids {
            if id.trim().is_empty() {
                return Err(AppError::InvalidArgument(
                    "来源引用 ID 不能为空，无法生成模型短标记".into(),
                ));
            }
            if !seen.insert(id.clone()) {
                continue;
            }
            let marker = loop {
                let candidate = format!("#{marker_index}");
                marker_index += 1;
                if !reserved_markers.contains(&candidate) {
                    break candidate;
                }
            };
            marker_by_source_id.insert(id.clone(), marker.clone());
            source_id_by_marker.insert(marker, id);
        }
        Ok(Self {
            marker_by_source_id,
            source_id_by_marker,
        })
    }

    pub(crate) fn marker_for(&self, source_id: &str) -> Result<&str, AppError> {
        self.marker_by_source_id
            .get(source_id)
            .map(String::as_str)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "source_span_id 不在本次模型请求候选中: {source_id}"
                ))
            })
    }

    pub(crate) fn mark_text(
        &self,
        content: &str,
        spans: &[SourceSpan],
    ) -> Result<String, AppError> {
        source_span_marker_text(content, spans, &self.marker_by_source_id)
            .map_err(AppError::InvalidArgument)
    }

    pub(crate) fn mark_excerpt(
        &self,
        content: &str,
        spans: &[SourceSpan],
    ) -> Result<String, AppError> {
        source_span_marker_excerpt(content, spans, &self.marker_by_source_id)
            .map_err(AppError::InvalidArgument)
    }

    pub(crate) fn mark_ranges(
        &self,
        content: &str,
        ranges: impl IntoIterator<Item = (String, usize, usize, String)>,
    ) -> Result<String, AppError> {
        source_marker_text(content, ranges, &self.marker_by_source_id)
            .map_err(AppError::InvalidArgument)
    }

    pub(crate) fn encode_input(&self, value: &Value) -> Result<Value, AppError> {
        let mut encoded = rewrite_source_references(value, self, &Direction::Encode, true)?;
        if let Value::Object(object) = &mut encoded {
            object.insert(
                "source_marker_protocol".into(),
                Value::String(SOURCE_MARKER_PROTOCOL.into()),
            );
        }
        Ok(encoded)
    }

    pub(crate) fn decode_output(&self, value: &Value) -> Result<Value, AppError> {
        rewrite_source_references(value, self, &Direction::Decode, false)
    }

    pub(crate) fn unmark_text(&self, text: &str) -> String {
        let bytes = text.as_bytes();
        let mut output = String::with_capacity(text.len());
        let mut copied_until = 0;
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] != b'#' {
                index += 1;
                continue;
            }
            let mut end = index + 1;
            while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
            if end == index + 1 || bytes.get(end) != Some(&b' ') {
                index += 1;
                continue;
            }
            let marker = &text[index..end];
            if self.source_id_by_marker.contains_key(marker) {
                output.push_str(&text[copied_until..index]);
                copied_until = end + 1;
                index = copied_until;
            } else {
                index = end + 1;
            }
        }
        output.push_str(&text[copied_until..]);
        output
    }

    fn decode_marker(&self, marker: &str) -> Result<String, AppError> {
        self.source_id_by_marker
            .get(marker)
            .cloned()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("模型返回了未提供的来源短标记: {marker}"))
            })
    }
}

fn collect_reference_ids(value: &Value, ids: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if SOURCE_REFERENCE_FIELDS.contains(&key.as_str()) {
                    match value {
                        Value::String(id) => ids.push(id.clone()),
                        Value::Array(values) => ids.extend(
                            values
                                .iter()
                                .filter_map(Value::as_str)
                                .map(ToOwned::to_owned),
                        ),
                        _ => {}
                    }
                } else if SOURCE_SPAN_RECORD_FIELDS.contains(&key.as_str()) {
                    if let Value::Array(records) = value {
                        ids.extend(records.iter().filter_map(|record| {
                            record
                                .get("id")
                                .and_then(Value::as_str)
                                .map(ToOwned::to_owned)
                        }));
                    }
                } else {
                    collect_reference_ids(value, ids);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_reference_ids(value, ids);
            }
        }
        _ => {}
    }
}

fn collect_value_marker_prefixes(value: &Value, reserved: &mut HashSet<String>) {
    match value {
        Value::String(text) => collect_marker_prefixes(text, reserved),
        Value::Array(values) => {
            for value in values {
                collect_value_marker_prefixes(value, reserved);
            }
        }
        Value::Object(object) => {
            for value in object.values() {
                collect_value_marker_prefixes(value, reserved);
            }
        }
        _ => {}
    }
}

fn collect_marker_prefixes(text: &str, reserved: &mut HashSet<String>) {
    let bytes = text.as_bytes();
    for (start, byte) in bytes.iter().enumerate() {
        if *byte != b'#' {
            continue;
        }
        let mut end = start + 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        for prefix_end in start + 2..=end {
            reserved.insert(text[start..prefix_end].to_string());
        }
    }
}

fn rewrite_source_references(
    value: &Value,
    markers: &SourceMarkerMap,
    direction: &Direction,
    strip_span_records: bool,
) -> Result<Value, AppError> {
    match value {
        Value::Object(object) => {
            let mut rewritten = Map::with_capacity(object.len());
            for (key, value) in object {
                if strip_span_records && SOURCE_SPAN_RECORD_FIELDS.contains(&key.as_str()) {
                    continue;
                }
                let value = if SOURCE_REFERENCE_FIELDS.contains(&key.as_str()) {
                    rewrite_reference_value(value, markers, direction)?
                } else {
                    rewrite_source_references(value, markers, direction, strip_span_records)?
                };
                rewritten.insert(key.clone(), value);
            }
            Ok(Value::Object(rewritten))
        }
        Value::Array(values) => values
            .iter()
            .map(|value| rewrite_source_references(value, markers, direction, strip_span_records))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        _ => Ok(value.clone()),
    }
}

fn rewrite_reference_value(
    value: &Value,
    markers: &SourceMarkerMap,
    direction: &Direction,
) -> Result<Value, AppError> {
    let rewrite = |id: &str| match direction {
        Direction::Encode => Ok(Value::String(markers.marker_for(id)?.to_string())),
        Direction::Decode => Ok(Value::String(markers.decode_marker(id)?)),
    };
    match value {
        Value::String(id) => rewrite(id),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::String(id) => rewrite(id),
                _ => Ok(value.clone()),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        _ => Ok(value.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn marker_map_encodes_inputs_and_decodes_nested_model_references() {
        let input = json!({
            "content": "第一段#0原文。\n第二段。",
            "source_spans": [{"id": "ch1:s1:f1", "text": "原文"}],
            "previous": {
                "source_span_ids": ["ch1:s1:f1", "ch1:s1:f2"],
                "allowed_source_span_ids": ["ch1:s1:f2"]
            }
        });
        let markers = SourceMarkerMap::from_ids_and_values(
            ["ch1:s1:f1".to_string(), "ch1:s1:f2".to_string()],
            std::iter::once(&input),
        )
        .unwrap();

        let encoded = markers.encode_input(&input).unwrap();
        assert_eq!(encoded["previous"]["source_span_ids"], json!(["#1", "#2"]));
        assert_eq!(
            encoded["previous"]["allowed_source_span_ids"],
            json!(["#2"])
        );
        assert!(encoded.get("source_spans").is_none());
        assert!(
            !encoded["source_marker_protocol"]
                .as_str()
                .unwrap()
                .contains('¶')
        );

        let decoded = markers
            .decode_output(&json!({
                "items": [{"source_span_ids": ["#2", "#1"]}],
                "source_span_id": null
            }))
            .unwrap();
        assert_eq!(
            decoded["items"][0]["source_span_ids"],
            json!(["ch1:s1:f2", "ch1:s1:f1"])
        );
        assert!(decoded["source_span_id"].is_null());
    }

    #[test]
    fn marker_map_rejects_unknown_and_legacy_model_ids() {
        let markers = SourceMarkerMap::from_ids(["ch1:s1:f1".to_string()]).unwrap();

        assert!(
            markers
                .decode_output(&json!({"source_span_ids": ["#1"]}))
                .is_err()
        );
        assert!(
            markers
                .decode_output(&json!({"source_span_ids": ["ch1:s1:f1"]}))
                .is_err()
        );
        assert!(
            markers
                .encode_input(&json!({"source_span_ids": ["ch1:s2:f1"]}))
                .is_err()
        );
    }

    #[test]
    fn marker_map_uses_deterministic_decimal_aliases() {
        let ids = (0..38)
            .map(|index| format!("span-{index}"))
            .collect::<Vec<_>>();
        let markers = SourceMarkerMap::from_ids(ids.clone()).unwrap();

        assert_eq!(markers.marker_for(&ids[0]).unwrap(), "#0");
        assert_eq!(markers.marker_for(&ids[35]).unwrap(), "#35");
        assert_eq!(markers.marker_for(&ids[36]).unwrap(), "#36");
        assert_eq!(markers.marker_for(&ids[37]).unwrap(), "#37");
    }

    #[test]
    fn marker_map_skips_aliases_that_appear_in_source_text() {
        let content = "#0开头，#1接着。";
        let spans = a4n_domain::source_spans::source_spans_for_content(7, 1, content);
        let markers = SourceMarkerMap::from_source_spans(&spans).unwrap();

        assert_eq!(markers.marker_for(&spans[0].id).unwrap(), "#2");
        assert_eq!(markers.marker_for(&spans[1].id).unwrap(), "#3");
        assert_eq!(
            markers.mark_text(content, &spans).unwrap(),
            "#2 #0开头，#3 #1接着。"
        );
    }

    #[test]
    fn marker_map_skips_aliases_in_nested_prompt_text() {
        let input = json!({"chapter": {"content": "正文引用#0，不是来源标记。"}});
        let markers = SourceMarkerMap::from_ids_and_values(
            ["ch1:s1:f1".to_string()],
            std::iter::once(&input),
        )
        .unwrap();

        assert_eq!(markers.marker_for("ch1:s1:f1").unwrap(), "#1");
    }

    #[test]
    fn marked_source_payload_is_smaller_than_full_span_records() {
        let content = (0..40)
            .map(|index| format!("人物{index}进入房间，然后转身离开。"))
            .collect::<Vec<_>>()
            .join("\n");
        let spans = a4n_domain::source_spans::source_spans_for_content(7, 1, &content);
        let markers = SourceMarkerMap::from_source_spans(&spans).unwrap();
        let legacy = json!({
            "content": content,
            "source_spans": spans.iter().map(|span| json!({
                "id": span.id,
                "paragraph_index": span.paragraph_index,
                "text": span.text,
            })).collect::<Vec<_>>()
        });
        let mut compact = legacy.clone();
        compact["content"] = Value::String(markers.mark_text(&content, &spans).unwrap());
        let compact = markers.encode_input(&compact).unwrap();

        assert!(
            serde_json::to_vec(&compact).unwrap().len()
                < serde_json::to_vec(&legacy).unwrap().len()
        );
    }
}
