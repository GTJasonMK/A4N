use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportTextProjectBlueprint {
    pub title: String,
    pub genre: String,
    pub style: Option<String>,
    pub tone: Option<String>,
    pub target_audience: Option<String>,
    pub one_sentence_summary: Option<String>,
    pub full_synopsis: Option<String>,
    pub world_setting: Option<Value>,
    pub current_version: String,
    pub needs_part_outlines: bool,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub avatar_svg: Option<String>,
    pub avatar_animal: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportTextProjectBlueprintVersion {
    pub version: String,
    pub change_description: Option<String>,
    pub blueprint_data: Value,
    pub world_setting_snapshot: Option<Value>,
    pub characters_snapshot: Option<Value>,
    pub relationships_snapshot: Option<Value>,
    pub synopsis_snapshot: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportTextProjectCharacter {
    pub name: String,
    pub character_type: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub personality: Option<String>,
    pub background: Option<String>,
    pub motivation: Option<String>,
    pub arc_summary: Option<String>,
    pub relationship_to_protagonist: Option<String>,
    pub arc_stages: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportTextProjectRelationship {
    pub relationship_type: String,
    pub character_a: String,
    pub character_b: String,
    pub description: Option<String>,
    pub strength: Option<i32>,
    pub evolution: Option<Value>,
}
