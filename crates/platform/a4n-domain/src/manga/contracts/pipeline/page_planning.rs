use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{MangaPageContentV1, MangaPageSlotV1, MangaPageSlotsV1};
use crate::media_authoring::AdaptationBeatRef;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageOutlineV1 {
    #[schemars(extend("const" = "manga_page_outline_v1"))]
    pub schema_version: String,
    #[schemars(length(min = 1), regex(pattern = r"\S"))]
    pub opening_state: String,
    #[schemars(length(min = 1, max = 100))]
    pub pages: Vec<MangaPageOutlineItemV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageOutlineItemV1 {
    #[schemars(range(min = 1, max = 100))]
    pub page_number: u32,
    #[schemars(length(min = 1), regex(pattern = r"\S"))]
    pub goal: String,
    pub source_span_ids: Vec<String>,
    #[schemars(length(min = 1))]
    pub beat_refs: Vec<AdaptationBeatRef>,
    #[schemars(length(min = 1), regex(pattern = r"\S"))]
    pub exit_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MangaPageContentDraftV1 {
    #[schemars(range(min = 1, max = 100))]
    pub page_number: u32,
    #[schemars(length(min = 1), regex(pattern = r"\S"))]
    pub summary: String,
    #[schemars(length(min = 1), regex(pattern = r"\S"))]
    pub action: String,
    #[schemars(schema_with = "nullable_nonempty_text")]
    pub reaction: crate::json_contract::RequiredNullable<String>,
    #[schemars(schema_with = "nullable_nonempty_text")]
    pub dialogue_intent: crate::json_contract::RequiredNullable<String>,
}

fn nullable_nonempty_text(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    serde_json::json!({"type":["string","null"],"minLength":1,"pattern":"\\S"})
        .try_into()
        .expect("static nullable text schema")
}

impl MangaPageOutlineV1 {
    pub fn validate(&self) -> Result<(), String> {
        crate::json_contract::OutputSchemaValidator::new(
            &crate::json_contract::typed_output_schema::<Self>(),
        )?
        .validate(&serde_json::to_value(self).map_err(|error| error.to_string())?)?;
        for (index, page) in self.pages.iter().enumerate() {
            super::validate_page_content_references(&page.source_span_ids, &page.beat_refs)
                .map_err(|error| format!("pages[{index}]: {error}"))?;
            if page.page_number as usize != index + 1 {
                return Err(format!("pages[{index}].page_number 必须从 1 连续递增"));
            }
        }
        Ok(())
    }

    pub fn entry_state(&self, index: usize) -> Result<&str, String> {
        self.pages
            .get(index)
            .ok_or_else(|| format!("页纲索引不存在：{index}"))?;
        Ok(if index == 0 {
            &self.opening_state
        } else {
            &self.pages[index - 1].exit_state
        })
    }

    pub fn page_slot(
        &self,
        index: usize,
        draft: MangaPageContentDraftV1,
    ) -> Result<MangaPageSlotV1, String> {
        let page = self
            .pages
            .get(index)
            .ok_or_else(|| format!("页纲索引不存在：{index}"))?;
        if draft.page_number != page.page_number {
            return Err(format!(
                "page_number 必须等于 {}，实际为 {}",
                page.page_number, draft.page_number
            ));
        }
        let content = MangaPageContentV1 {
            summary: draft.summary,
            action: draft.action,
            reaction: draft.reaction.0,
            dialogue_intent: draft.dialogue_intent.0,
            source_span_ids: page.source_span_ids.clone(),
            beat_refs: page.beat_refs.clone(),
            entry_state: self.entry_state(index)?.into(),
            exit_state: page.exit_state.clone(),
        };
        content.validate()?;
        Ok(MangaPageSlotV1 {
            page_number: page.page_number,
            content,
        })
    }

    pub fn assemble(
        &self,
        drafts: Vec<MangaPageContentDraftV1>,
    ) -> Result<MangaPageSlotsV1, String> {
        if drafts.len() != self.pages.len() {
            return Err(format!(
                "页面内容未完成：期望 {} 页，实际 {} 页",
                self.pages.len(),
                drafts.len()
            ));
        }
        let slots = MangaPageSlotsV1 {
            schema_version: super::MangaPageSlotsSchemaVersion::V1,
            pages: drafts
                .into_iter()
                .enumerate()
                .map(|(index, draft)| self.page_slot(index, draft))
                .collect::<Result<_, _>>()?,
        };
        slots.validate()?;
        Ok(slots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn outline() -> MangaPageOutlineV1 {
        serde_json::from_value(json!({
            "schema_version":"manga_page_outline_v1", "opening_state":"站在门外",
            "pages":[
                {"page_number":1, "goal":"发现密信", "source_span_ids":["source-1"], "beat_refs":[{"unit_id":"station","beat_id":"evidence"}], "exit_state":"发现密信"},
                {"page_number":2, "goal":"决定追查", "source_span_ids":[], "beat_refs":[{"unit_id":"station","beat_id":"evidence"}], "exit_state":"进入机房"}
            ]
        })).unwrap()
    }

    fn draft(page: u32) -> MangaPageContentDraftV1 {
        serde_json::from_value(json!({"page_number":page,"summary":"发现线索并追查","action":"查看密信","reaction":null,"dialogue_intent":null})).unwrap()
    }

    #[test]
    fn boundaries_and_assignments_have_one_owner() {
        let outline = outline();
        outline.validate().unwrap();
        let slots = outline.assemble(vec![draft(1), draft(2)]).unwrap();
        assert_eq!(slots.pages[0].content.entry_state, outline.opening_state);
        assert_eq!(
            slots.pages[1].content.entry_state,
            slots.pages[0].content.exit_state
        );
        assert_eq!(
            slots.pages[0].content.source_span_ids,
            outline.pages[0].source_span_ids
        );
        assert_eq!(slots.pages[1].content.beat_refs, outline.pages[1].beat_refs);
        assert!(slots.pages[1].content.source_span_ids.is_empty());
        let mut inconsistent = slots;
        inconsistent.pages[1].content.entry_state = "凭空换了地点".into();
        assert!(inconsistent.validate().unwrap_err().contains("上一页"));
    }

    #[test]
    fn rejects_partial_wrong_page_and_blank_optional_content() {
        let outline = outline();
        assert!(outline.assemble(vec![draft(1)]).is_err());
        assert!(outline.assemble(vec![draft(2), draft(1)]).is_err());
        assert!(outline.entry_state(2).is_err());
        let mut invalid = draft(1);
        invalid.reaction.0 = Some(" ".into());
        assert!(outline.page_slot(0, invalid).is_err());
    }

    #[test]
    fn nullable_fields_must_be_present_in_both_schema_and_deserializer() {
        let validator = crate::json_contract::OutputSchemaValidator::new(
            &crate::json_contract::typed_output_schema::<MangaPageContentDraftV1>(),
        )
        .unwrap();
        let valid = serde_json::to_value(draft(1)).unwrap();
        validator.validate(&valid).unwrap();
        for field in ["reaction", "dialogue_intent"] {
            let mut invalid = valid.clone();
            invalid.as_object_mut().unwrap().remove(field);
            assert!(validator.validate(&invalid).is_err());
            assert!(serde_json::from_value::<MangaPageContentDraftV1>(invalid).is_err());
        }
        for extra in ["entry_state", "beat_refs", "pages"] {
            let mut invalid = valid.clone();
            invalid[extra] = json!("unowned field");
            assert!(validator.validate(&invalid).is_err());
            assert!(serde_json::from_value::<MangaPageContentDraftV1>(invalid).is_err());
        }
    }

    #[test]
    fn outline_rejects_invalid_metadata_but_allows_intentional_cross_page_beats() {
        let valid = outline();
        valid.validate().unwrap();
        for invalid in [
            MangaPageOutlineV1 {
                pages: vec![],
                ..valid.clone()
            },
            MangaPageOutlineV1 {
                opening_state: " ".into(),
                ..valid.clone()
            },
            MangaPageOutlineV1 {
                schema_version: "old".into(),
                ..valid.clone()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
        let mut invalid = valid.clone();
        invalid.pages[1].page_number = 3;
        assert!(invalid.validate().is_err());
        let mut invalid = valid.clone();
        invalid.pages[0].source_span_ids.push("source-1".into());
        assert!(invalid.validate().is_err());
        let mut invalid = valid;
        let duplicate = invalid.pages[0].beat_refs[0].clone();
        invalid.pages[0].beat_refs.push(duplicate);
        assert!(invalid.validate().is_err());
    }
}
