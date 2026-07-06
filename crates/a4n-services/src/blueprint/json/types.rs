use serde_json::Value;

#[derive(Clone, Debug)]
pub(in crate::blueprint) struct ParsedBlueprint {
    pub(in crate::blueprint) raw: Value,
    pub(in crate::blueprint) title: String,
    pub(in crate::blueprint) genre: String,
    pub(in crate::blueprint) style: String,
    pub(in crate::blueprint) tone: String,
    pub(in crate::blueprint) target_audience: String,
    pub(in crate::blueprint) one_sentence_summary: String,
    pub(in crate::blueprint) full_synopsis: String,
    pub(in crate::blueprint) world_setting: Value,
    pub(in crate::blueprint) total_chapters: i32,
    pub(in crate::blueprint) needs_part_outlines: bool,
    pub(in crate::blueprint) chapters_per_part: i32,
    pub(in crate::blueprint) characters: Vec<ParsedBlueprintCharacter>,
    pub(in crate::blueprint) relationships: Vec<ParsedBlueprintRelationship>,
}

#[derive(Clone, Debug)]
pub(in crate::blueprint) struct ParsedBlueprintCharacter {
    pub(in crate::blueprint) name: String,
    pub(in crate::blueprint) character_type: String,
    pub(in crate::blueprint) gender: Option<String>,
    pub(in crate::blueprint) age: Option<String>,
    pub(in crate::blueprint) appearance: String,
    pub(in crate::blueprint) personality: String,
    pub(in crate::blueprint) background: String,
    pub(in crate::blueprint) motivation: String,
    pub(in crate::blueprint) arc_summary: String,
    pub(in crate::blueprint) relationship_to_protagonist: String,
    pub(in crate::blueprint) arc_stages: Option<Value>,
}

#[derive(Clone, Debug)]
pub(in crate::blueprint) struct ParsedBlueprintRelationship {
    pub(in crate::blueprint) relationship_type: String,
    pub(in crate::blueprint) character_a: String,
    pub(in crate::blueprint) character_b: String,
    pub(in crate::blueprint) description: String,
    pub(in crate::blueprint) strength: Option<i32>,
    pub(in crate::blueprint) evolution: Option<Value>,
}

#[derive(Clone, Debug)]
pub(in crate::blueprint) struct ParsedBlueprintOptimization {
    pub(in crate::blueprint) raw: Value,
    pub(in crate::blueprint) style: Option<String>,
    pub(in crate::blueprint) full_synopsis: Option<String>,
    pub(in crate::blueprint) world_setting: Option<Value>,
    pub(in crate::blueprint) characters: Option<Vec<ParsedBlueprintCharacter>>,
    pub(in crate::blueprint) relationships: Option<Vec<ParsedBlueprintRelationship>>,
}
