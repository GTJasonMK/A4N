use serde_json::Value;

#[derive(Clone, Debug)]
pub struct ParsedBlueprint {
    pub raw: Value,
    pub title: String,
    pub genre: String,
    pub style: String,
    pub tone: String,
    pub target_audience: String,
    pub one_sentence_summary: String,
    pub full_synopsis: String,
    pub world_setting: Value,
    pub total_chapters: i32,
    pub needs_part_outlines: bool,
    pub chapters_per_part: i32,
    pub characters: Vec<ParsedBlueprintCharacter>,
    pub relationships: Vec<ParsedBlueprintRelationship>,
}

#[derive(Clone, Debug)]
pub struct ParsedBlueprintCharacter {
    pub name: String,
    pub character_type: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: String,
    pub personality: String,
    pub background: String,
    pub motivation: String,
    pub arc_summary: String,
    pub relationship_to_protagonist: String,
    pub arc_stages: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct ParsedBlueprintRelationship {
    pub relationship_type: String,
    pub character_a: String,
    pub character_b: String,
    pub description: String,
    pub strength: Option<i32>,
    pub evolution: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct ParsedBlueprintOptimization {
    pub raw: Value,
    pub style: Option<String>,
    pub full_synopsis: Option<String>,
    pub world_setting: Option<Value>,
    pub characters: Option<Vec<ParsedBlueprintCharacter>>,
    pub relationships: Option<Vec<ParsedBlueprintRelationship>>,
}
