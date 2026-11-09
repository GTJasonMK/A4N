use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MangaPreprocessingStep {
    Facts,
    Speech,
    Visuals,
    Synthesis,
}

impl MangaPreprocessingStep {
    pub const ALL: [Self; 4] = [Self::Facts, Self::Speech, Self::Visuals, Self::Synthesis];

    pub const fn prompt_name(self) -> &'static str {
        match self {
            Self::Facts => "manga_preprocessing_facts",
            Self::Speech => "manga_preprocessing_speech",
            Self::Visuals => "manga_preprocessing_visuals",
            Self::Synthesis => "manga_preprocessing_synthesis",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Facts => "原文事件与人物",
            Self::Speech => "对白与旁白",
            Self::Visuals => "场景与道具",
            Self::Synthesis => "章节综合",
        }
    }

    pub const fn order(self) -> i32 {
        match self {
            Self::Facts => 0,
            Self::Speech => 1,
            Self::Visuals => 2,
            Self::Synthesis => 3,
        }
    }

    pub fn call_key(self, chapter_id: i64) -> String {
        format!("manga_pipeline:{chapter_id}:{}:v1", self.prompt_name())
    }

    pub fn output_schema(self) -> Value {
        use crate::json_contract::typed_output_schema;
        match self {
            Self::Facts => typed_output_schema::<MangaPreprocessingFacts>(),
            Self::Speech => typed_output_schema::<MangaPreprocessingSpeech>(),
            Self::Visuals => typed_output_schema::<MangaPreprocessingVisuals>(),
            Self::Synthesis => typed_output_schema::<MangaPreprocessingSynthesis>(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPreprocessingFacts {
    #[schemars(schema_with = "preprocessing_schema::source_events")]
    pub source_events: Value,
    #[schemars(schema_with = "preprocessing_schema::extracted_characters")]
    pub characters: Value,
}

impl MangaPreprocessingFacts {
    pub fn validate(&self) -> Result<(), String> {
        validate_step_schema(self)?;
        preprocessing_event_ids(&self.source_events)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPreprocessingSpeech {
    #[schemars(schema_with = "preprocessing_schema::extracted_speech")]
    pub speech: Value,
}

impl MangaPreprocessingSpeech {
    pub fn validate(&self, facts: &MangaPreprocessingFacts) -> Result<(), String> {
        validate_step_schema(self)?;
        validate_preprocessing_speech(
            &self.speech,
            &preprocessing_event_ids(&facts.source_events)?,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPreprocessingVisuals {
    #[schemars(schema_with = "preprocessing_schema::extracted_visual_anchors")]
    pub visual_anchors: Value,
    #[schemars(schema_with = "preprocessing_schema::extracted_items")]
    pub items: Value,
}

impl MangaPreprocessingVisuals {
    pub fn validate(&self, facts: &MangaPreprocessingFacts) -> Result<(), String> {
        validate_step_schema(self)?;
        validate_preprocessing_visuals(
            &self.visual_anchors,
            &self.items,
            &preprocessing_event_ids(&facts.source_events)?,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPreprocessingSynthesis {
    pub comic_synthesis: MangaChapterComicSynthesisV1,
}

impl MangaPreprocessingSynthesis {
    pub fn validate(&self) -> Result<(), String> {
        validate_step_schema(self)
    }
}

fn validate_step_schema<T: Serialize + JsonSchema>(value: &T) -> Result<(), String> {
    crate::json_contract::OutputSchemaValidator::new(
        &crate::json_contract::typed_output_schema::<T>(),
    )?
    .validate(&serde_json::to_value(value).map_err(|error| error.to_string())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> MangaPreprocessingFacts {
        MangaPreprocessingFacts {
            source_events: json!({"events":[{"event_id":"event-1","type":"action","description":"推门","participants":[],"source_span_ids":["span-1"]}]}),
            characters: json!({}),
        }
    }

    #[test]
    fn steps_accept_only_their_fields_and_the_same_event_index() {
        let facts = facts();
        facts.validate().unwrap();
        let speech = MangaPreprocessingSpeech {
            speech: json!({"dialogues":[],"narrations":[]}),
        };
        speech.validate(&facts).unwrap();
        let visuals = MangaPreprocessingVisuals {
            visual_anchors: json!({"scenes":[]}),
            items: json!({"items":[]}),
        };
        visuals.validate(&facts).unwrap();
        let summary = MangaPreprocessingSynthesis {
            comic_synthesis: MangaChapterComicSynthesisV1 {
                chapter_summary: "人物推门进入".into(),
                mood_progression: vec!["犹豫".into(), "靠近".into(), "进入".into()],
            },
        };
        summary.validate().unwrap();
        for (step, mut output) in MangaPreprocessingStep::ALL.into_iter().zip([
            serde_json::to_value(&facts).unwrap(),
            serde_json::to_value(&speech).unwrap(),
            serde_json::to_value(&visuals).unwrap(),
            serde_json::to_value(&summary).unwrap(),
        ]) {
            let validator =
                crate::json_contract::OutputSchemaValidator::new(&step.output_schema()).unwrap();
            validator.validate(&output).unwrap();
            output["schema_version"] = json!(MANGA_PREPROCESSING_V3);
            assert!(
                validator.validate(&output).is_err(),
                "{} must not accept an entire bundle",
                step.prompt_name()
            );
        }
    }

    #[test]
    fn source_and_event_references_and_required_nested_fields_are_not_filled_in() {
        let mut facts = facts();
        let duplicate = facts.source_events["events"][0].clone();
        facts.source_events["events"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(facts.validate().unwrap_err().contains("不能重复"));
        let facts = self::facts();
        let mut speech = MangaPreprocessingSpeech {
            speech: json!({"dialogues":[{
            "index":0,"speaker":"旅人","content":"你好","source_event_id":"unknown","emotion":"平静",
            "target":null,"is_internal":false,"bubble_type":"normal"
        }],"narrations":[]}),
        };
        assert!(
            speech
                .validate(&facts)
                .unwrap_err()
                .contains("不存在的事件")
        );
        speech.speech["dialogues"][0]["source_event_id"] = json!("event-1");
        speech.validate(&facts).unwrap();
        speech.speech["dialogues"][0]
            .as_object_mut()
            .unwrap()
            .remove("speaker");
        assert!(speech.validate(&facts).is_err());
        let visuals = MangaPreprocessingVisuals {
            visual_anchors: json!({"scenes":[]}),
            items: json!({"items":[{
                "name":"信","description":"信封","description_en":"letter","importance":"prop",
                "first_appearance_source_event_id":"unknown","visual_features":"手中信封","source_span_ids":["span-1"]
            }]}),
        };
        assert!(
            visuals
                .validate(&facts)
                .unwrap_err()
                .contains("不存在的事件")
        );
    }
}
