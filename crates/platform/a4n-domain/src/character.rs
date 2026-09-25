use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

pub const CHARACTER_DELETION_THRESHOLD: i32 = 5;

pub fn initial_snapshot_data() -> Value {
    json!({
        "synced_chapter": 0,
        "state_summary": "初始空状态",
        "changes_in_chapter": 0,
        "behaviors_in_chapter": 0,
        "attributes": {
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
        },
        "explicit_attributes": {},
        "implicit_attributes": {},
        "social_attributes": {},
    })
}

pub fn category_storage_key(category: &str) -> &'static str {
    match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        _ => "attributes",
    }
}

pub fn apply_attribute_to_state(
    state: &mut Value,
    category: &str,
    key: &str,
    operation: &str,
    new_value: Value,
) -> Result<(), String> {
    let storage_key = category_storage_key(category);
    let root = state
        .as_object_mut()
        .ok_or_else(|| "角色当前状态不是 JSON 对象".to_string())?;
    let attributes = root
        .get_mut(storage_key)
        .ok_or_else(|| format!("角色当前状态缺少 {storage_key}"))?
        .as_object_mut()
        .ok_or_else(|| format!("角色当前状态 {storage_key} 不是 JSON 对象"))?;
    if operation == "delete" {
        attributes.remove(key);
    } else {
        attributes.insert(key.to_string(), new_value);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterPortraitProfile {
    pub name: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
    pub base_attributes: Value,
    pub current_state: Value,
    pub character_arc: Option<Value>,
    pub voice_fingerprint: Option<Value>,
}

const CHARACTER_SHEET_IMAGE_GOAL: &str = "单张人物三视图与人物拆解设定图，画面必须同时展示同一角色的正面、侧面、背面全身形象，以及独立的人物拆解区";
const REQUIRED_VIEWS: [&str; 3] = ["正面全身视图", "侧面全身视图", "背面全身视图"];
const BREAKDOWN_SECTIONS: [&str; 8] = [
    "服饰层级拆解：外套、内搭、领口、袖口、腰部、衣摆、下装的结构关系",
    "发型拆解：正面刘海、侧面层次、背面发长和发饰；没有发饰则只拆发型轮廓",
    "鞋履拆解：鞋型、靴筒、鞋底、鞋带或扣件、材质和磨损",
    "材质与色板拆解：主色、辅色、点缀色，以及布料、皮革、金属等材质样片",
    "纹样与身份符号拆解：档案明确出现的徽记、纹样、标志、制服元素或阵营符号",
    "已存在配饰拆解：只拆档案明确出现的项链、耳饰、戒指、手套、胸针、徽章等",
    "已存在随身物件拆解：只拆档案明确出现的包、武器、工具、法器、挂件等",
    "背面结构拆解：后领、背部装饰、腰部结构、后摆、背包或披风等背面可见元素",
];
const LAYOUT_SPEC: [&str; 6] = [
    "横向角色设定图画布，左侧或上方约三分之二区域放三视图主体",
    "右侧或下方约三分之一区域放人物拆解区，拆解区与三视图同图出现",
    "人物拆解区至少包含服饰层级、发型、鞋履、材质色板四类视觉拆解",
    "配饰和随身物件只在档案明确出现时进入拆解区，没有则不要编造",
    "拆解区使用局部放大框、分离小图、材质样片、色块和短标签，不依赖长文字",
    "三视图主体和拆解区必须保持同一角色、同一服装、同一配色和同一批已存在配饰",
];
const DETAIL_FOCUS: [&str; 12] = [
    "发型、刘海、发色、发饰和头部轮廓",
    "脸型、五官、瞳色、肤色、表情和年龄感",
    "上装、外套、内搭、领口、袖口、衣摆和层次关系",
    "下装、腰带、裙摆或裤型、褶皱、缝线和轮廓比例",
    "鞋子、靴筒、鞋底、鞋带、鞋扣和磨损细节",
    "档案明确出现的项链、耳饰、戒指、手链、手套、胸针、徽章等配饰",
    "档案明确出现的包、武器、工具、法器、挂件等随身物件",
    "布料、皮革、金属、宝石、木石等材质质感",
    "主色、辅色、点缀色、图案、纹样和身份符号",
    "正面能看到的服装结构和配饰摆放",
    "侧面能看到的厚度、发型层次、衣物廓形和鞋型",
    "背面能看到的发长、后领、背部装饰、腰部结构和后摆",
];
const COMPOSITION_CONSTRAINTS: [&str; 8] = [
    "三视图主体必须位于同一张图中，从左到右清晰排列为正面、侧面、背面",
    "人物拆解区必须与三视图同图呈现，可放在右侧或下方，用局部放大框、拆件小图、材质色板或简短标注展示",
    "最终图像不能只有三视图，必须能看出独立的人物拆解区和至少四类拆解小组件",
    "三种视角必须是同一角色、同一服装、同一批已存在配饰、同一比例和同一色彩方案",
    "使用中性站姿，手臂可略微离开身体以展示袖口、腰线、手部细节和侧面轮廓；如角色确有戒指或手套再展示对应细节",
    "角色全身完整入画，从头顶到鞋底都不能裁切",
    "背景保持干净，优先白底、浅灰底或透明感设定图背景，不加入剧情场景",
    "拆解区不能破坏三视图主体比例，不能把角色拆成多套不同服装或不同身份版本",
];
const NEGATIVE_CONSTRAINTS: [&str; 8] = [
    "避免单人半身像或普通立绘构图",
    "避免只有一个角度、动态战斗姿势或强透视镜头",
    "避免背景场景喧宾夺主",
    "避免正面、侧面、背面的服装和配饰不一致",
    "避免裁切头发、手部、脚部或鞋子",
    "避免多角色、额外肢体、错位手指和重复脸部",
    "避免长段文字、水印、乱码标签和无意义符号；如使用标注，只使用短标签，不依赖文字传达关键信息",
    "避免把配饰描述成抽象气质而不落到具体形态",
];

pub fn profile_portrait_seed_prompt(profile: &CharacterPortraitProfile) -> Result<String, String> {
    let mut parts = vec![
        character_sheet_seed_instruction(),
        format!("角色名：{}", profile.name),
    ];
    for (label, value) in [
        ("性别", profile.gender.as_deref()),
        ("年龄", profile.age.as_deref()),
        ("外观", profile.appearance.as_deref()),
        ("性格", profile.personality.as_deref()),
        ("背景", profile.background.as_deref()),
    ] {
        if let Some(text) = optional_profile_text(value) {
            parts.push(format!("{label}：{text}"));
        }
    }
    let profile_context = portrait_profile_context(profile);
    if matches!(profile_context.as_object(), Some(fields) if !fields.is_empty()) {
        let context = serde_json::to_string(&profile_context).map_err(|err| err.to_string())?;
        parts.push(format!("角色档案上下文：{context}"));
    }
    Ok(parts.join("，"))
}

pub fn optional_profile_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub fn expression_or_default(expression: Option<&str>) -> String {
    expression
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("neutral")
        .to_string()
}

pub fn portrait_style_with_pose(style: Option<&str>, pose: Option<&str>) -> Option<String> {
    let style = style.map(str::trim).filter(|value| !value.is_empty());
    let pose = pose.map(str::trim).filter(|value| !value.is_empty());
    match (style, pose) {
        (Some(style), Some(pose)) => Some(format!("{style}\n姿势要求：{pose}")),
        (Some(style), None) => Some(style.to_string()),
        (None, Some(pose)) => Some(format!("姿势要求：{pose}")),
        (None, None) => None,
    }
}

pub fn portrait_profile_context(profile: &CharacterPortraitProfile) -> Value {
    let mut object = Map::new();
    insert_meaningful_json(&mut object, "base_attributes", &profile.base_attributes);
    insert_meaningful_json(&mut object, "current_state", &profile.current_state);
    if let Some(arc) = profile.character_arc.as_ref() {
        insert_meaningful_json(&mut object, "character_arc", arc);
    }
    if let Some(voice) = profile.voice_fingerprint.as_ref() {
        insert_meaningful_json(&mut object, "voice_fingerprint", voice);
    }
    Value::Object(object)
}

fn insert_meaningful_json(object: &mut Map<String, Value>, key: &str, value: &Value) {
    if is_meaningful_json(value) {
        object.insert(key.to_string(), value.clone());
    }
}

fn is_meaningful_json(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
    }
}

fn character_sheet_seed_instruction() -> String {
    let views = REQUIRED_VIEWS.join("、");
    let breakdowns = BREAKDOWN_SECTIONS.join("；");
    let layout = LAYOUT_SPEC.join("；");
    let details = DETAIL_FOCUS.join("；");
    let composition = COMPOSITION_CONSTRAINTS.join("；");
    let negatives = NEGATIVE_CONSTRAINTS.join("；");
    format!(
        "单张人物三视图与人物拆解设定图，{goal}。必须包含：{views}。必须额外包含人物拆解区：{breakdowns}。版式要求：{layout}。细节重点：{details}。细节策略：只强制保留角色档案、外观描述或上下文中已经明确出现的身体、服饰、配饰和随身物件；缺失字段不要编造为事实，不要硬塞项链、戒指、武器等具体配饰。构图约束：{composition}。避免项：{negatives}。",
        goal = CHARACTER_SHEET_IMAGE_GOAL
    )
}

pub fn character_sheet_generation_fields() -> Map<String, Value> {
    let mut fields = Map::new();
    fields.insert(
        "image_goal".to_string(),
        Value::String(CHARACTER_SHEET_IMAGE_GOAL.to_string()),
    );
    fields.insert("required_views".to_string(), json!(REQUIRED_VIEWS));
    fields.insert("breakdown_sections".to_string(), json!(BREAKDOWN_SECTIONS));
    fields.insert("layout_spec".to_string(), json!(LAYOUT_SPEC));
    fields.insert("detail_focus".to_string(), json!(DETAIL_FOCUS));
    fields.insert(
        "composition_constraints".to_string(),
        json!(COMPOSITION_CONSTRAINTS),
    );
    fields.insert(
        "detail_policy".to_string(),
        Value::String(
            "只强制保留角色档案、外观描述或上下文中已经明确出现的身体、服饰、配饰和随身物件；缺失字段不要编造为事实。可以根据身份、时代和风格补充少量不冲突的基础服装结构，但不要硬塞项链、戒指、武器等具体配饰。".to_string(),
        ),
    );
    fields.insert(
        "negative_constraints".to_string(),
        json!(NEGATIVE_CONSTRAINTS),
    );
    fields
}

pub fn character_identity_key(name: &str) -> String {
    name.trim().to_lowercase()
}

pub fn analysis_sync_character_names(
    analysis_character_names: &BTreeSet<String>,
    discovery_character_names: &BTreeSet<String>,
) -> BTreeSet<String> {
    analysis_character_names
        .iter()
        .chain(discovery_character_names.iter())
        .cloned()
        .collect()
}

pub fn character_keys_from_names(names: &BTreeSet<String>) -> BTreeSet<String> {
    names
        .iter()
        .map(|name| character_identity_key(name))
        .filter(|key| !key.is_empty())
        .collect()
}

pub fn character_names_from_chapter_analysis(analysis: &Value) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    if let Some(metadata) = analysis.get("metadata") {
        collect_character_name_values(metadata.get("characters"), &mut names);
    }
    if let Some(states) = analysis.get("character_states").and_then(Value::as_object) {
        for name in states.keys() {
            insert_character_name(name, &mut names);
        }
    }
    if let Some(events) = analysis.get("key_events").and_then(Value::as_array) {
        for event in events {
            collect_character_name_values(event.get("involved_characters"), &mut names);
        }
    }
    names
}

pub fn character_names_from_discovery_result(result: &Value) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for field in ["created", "existing"] {
        let Some(items) = result.get(field).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            collect_character_name_values(item.get("name"), &mut names);
            collect_character_name_values(item.get("aliases"), &mut names);
            if let Some(base_attributes) = item.get("base_attributes") {
                collect_character_attribute_aliases(base_attributes, &mut names);
            }
            if let Some(current_state) = item.get("current_state") {
                collect_character_attribute_aliases(current_state, &mut names);
            }
        }
    }
    names
}

fn collect_character_attribute_aliases(attributes: &Value, names: &mut BTreeSet<String>) {
    if let Some(explicit) = attributes.get("explicit_attributes") {
        collect_character_name_values(explicit.get("aliases"), names);
    }
}

fn collect_character_name_values(value: Option<&Value>, names: &mut BTreeSet<String>) {
    match value {
        Some(Value::String(text)) => insert_character_name(text, names),
        Some(Value::Array(items)) => {
            for item in items {
                collect_character_name_values(Some(item), names);
            }
        }
        _ => {}
    }
}

fn insert_character_name(name: &str, names: &mut BTreeSet<String>) {
    let name = name.trim();
    if !name.is_empty() {
        names.insert(name.to_string());
    }
}

#[cfg(test)]
mod portrait_tests {
    use super::*;

    fn profile() -> CharacterPortraitProfile {
        CharacterPortraitProfile {
            name: "陈恪".to_string(),
            gender: Some("男".to_string()),
            age: Some("二十余岁".to_string()),
            appearance: Some("清瘦".to_string()),
            background: Some("隋唐史博士".to_string()),
            personality: Some("冷静".to_string()),
            base_attributes: json!({"explicit_attributes":{"身份":"东宫书吏"}}),
            current_state: json!({"explicit_attributes":{"衣着":"青色官袍"}}),
            character_arc: Some(json!({"current":"试图改变玄武门结局"})),
            voice_fingerprint: None,
        }
    }

    #[test]
    fn portrait_prompt_keeps_explicit_profile_context() {
        let prompt = profile_portrait_seed_prompt(&profile()).expect("prompt should serialize");
        assert!(prompt.contains("角色名：陈恪"));
        assert!(prompt.contains("东宫书吏"));
        assert!(prompt.contains("青色官袍"));
        assert!(prompt.contains("至少包含服饰层级、发型、鞋履、材质色板四类视觉拆解"));
    }

    #[test]
    fn portrait_context_excludes_database_identity() {
        let context = portrait_profile_context(&profile());
        let text = serde_json::to_string(&context).expect("context should serialize");
        assert!(text.contains("玄武门"));
        assert!(!text.contains("project_id"));
        assert!(!text.contains("created_at"));
    }
}

#[cfg(test)]
mod character_name_tests {
    use super::*;

    #[test]
    fn character_name_projection_merges_analysis_and_discovery_aliases() {
        let analysis = json!({
            "metadata": {"characters": [" 林烬 "]},
            "character_states": {"苏晚": {}},
            "key_events": [{"involved_characters": ["林烬", "顾遥"]}]
        });
        let discovery = json!({
            "created": [{
                "name": "林烬",
                "aliases": ["阿烬"],
                "base_attributes": {"explicit_attributes": {"aliases": ["小烬"]}}
            }]
        });

        let analysis_names = character_names_from_chapter_analysis(&analysis);
        let discovery_names = character_names_from_discovery_result(&discovery);
        let names = analysis_sync_character_names(&analysis_names, &discovery_names);

        assert_eq!(
            names,
            ["林烬", "苏晚", "顾遥", "阿烬", "小烬"]
                .into_iter()
                .map(str::to_string)
                .collect()
        );
        assert_eq!(
            character_keys_from_names(&names),
            ["林烬", "苏晚", "顾遥", "阿烬", "小烬"]
                .into_iter()
                .map(str::to_string)
                .collect()
        );
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterProfileCreateInput {
    pub project_id: i64,
    pub name: String,
    pub base_attributes: Value,
    pub current_state: Value,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterPortraitCreateInput {
    pub profile_id: i64,
    pub project_id: i64,
    pub portrait_type: String,
    pub image_path: String,
    pub prompt: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterSyncChapterStateRecordInput {
    pub chapter_number: i32,
    pub state_summary: String,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
    pub emotional_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterAttributeValueChangeInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub category: String,
    pub key: String,
    pub value: Value,
    pub change_description: Option<String>,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone)]
pub struct CharacterAttributeDeletionRequestInput {
    pub profile_id: i64,
    pub category: String,
    pub key: String,
    pub chapter_number: i32,
    pub reason: String,
    pub evidence: String,
}

#[derive(Debug, Clone)]
pub struct CharacterAttributeDeletionExecutionInput {
    pub profile_id: i64,
    pub category: String,
    pub key: String,
    pub event_cause: String,
    pub evidence: String,
    pub chapter_number: i32,
}

#[derive(Debug, Clone)]
pub struct CharacterImplicitReviewDecisionRecordInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub attribute_key: String,
    pub current_value: Value,
    pub decision: String,
    pub suggested_new_value: Option<Value>,
    pub reasoning: String,
    pub evidence_summary: String,
}

#[derive(Debug, Clone)]
pub struct InsertBehaviorRecordInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub source_kind: String,
    pub behavior_type: String,
    pub behavior_description: String,
    pub original_text: String,
    pub behavior_tags: Value,
    pub classification_results: Value,
    pub related_characters: Option<Value>,
    pub impact_analysis: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ForeshadowingStatus {
    Pending,
    PartiallyRevealed,
    Resolved,
    Abandoned,
}

impl ForeshadowingStatus {
    pub const ALL: [Self; 4] = [
        Self::Pending,
        Self::PartiallyRevealed,
        Self::Resolved,
        Self::Abandoned,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::PartiallyRevealed => "partially_revealed",
            Self::Resolved => "resolved",
            Self::Abandoned => "abandoned",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }

    pub const fn can_resolve(self) -> bool {
        matches!(self, Self::Pending | Self::PartiallyRevealed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForeshadowingRecordInput {
    pub description: String,
    #[schemars(range(min = 1))]
    pub setup_chapter: i32,
    #[schemars(range(min = 1))]
    #[serde(default)]
    pub payoff_chapter: Option<i32>,
    pub status: ForeshadowingStatus,
    #[serde(default)]
    pub related_characters: Vec<String>,
    #[schemars(range(min = 1, max = 5))]
    pub importance: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CharacterAttributeCategory {
    Explicit,
    Implicit,
    Social,
}

impl CharacterAttributeCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Implicit => "implicit",
            Self::Social => "social",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "explicit" => Some(Self::Explicit),
            "implicit" => Some(Self::Implicit),
            "social" => Some(Self::Social),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CharacterAttributeOperation {
    Add,
    Modify,
    Delete,
}

impl CharacterAttributeOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Modify => "modify",
            Self::Delete => "delete",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "add" => Some(Self::Add),
            "modify" => Some(Self::Modify),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CharacterTimelineSourceKind {
    User,
    AiSync,
    TextFact,
    LegacyUnknown,
}

impl CharacterTimelineSourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::AiSync => "ai_sync",
            Self::TextFact => "text_fact",
            Self::LegacyUnknown => "legacy_unknown",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "ai_sync" => Some(Self::AiSync),
            "text_fact" => Some(Self::TextFact),
            "legacy_unknown" => Some(Self::LegacyUnknown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeletionMarkStatus {
    All,
    Pending,
    Ready,
    Executed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImplicitUpdateDecision {
    Keep,
    Modify,
    Delete,
}

impl ImplicitUpdateDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Modify => "modify",
            Self::Delete => "delete",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "keep" => Some(Self::Keep),
            "modify" => Some(Self::Modify),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AppliedImplicitUpdateAction {
    Keep,
    Modify,
    MarkDeletion,
}

#[cfg(test)]
mod tests {
    use super::{ForeshadowingStatus, apply_attribute_to_state};
    use serde_json::json;

    #[test]
    fn attribute_state_projection_applies_and_deletes_values() {
        let mut state = json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        });

        apply_attribute_to_state(
            &mut state,
            "explicit",
            "appearance",
            "modify",
            json!("白衣"),
        )
        .expect("canonical state should accept attribute changes");
        assert_eq!(state["explicit_attributes"]["appearance"], json!("白衣"));

        apply_attribute_to_state(&mut state, "explicit", "appearance", "delete", json!(null))
            .expect("canonical state should accept attribute deletion");
        assert!(state["explicit_attributes"].get("appearance").is_none());
    }

    #[test]
    fn attribute_state_projection_rejects_non_object_state() {
        let mut state = json!("invalid");

        let error = apply_attribute_to_state(
            &mut state,
            "explicit",
            "appearance",
            "modify",
            json!("白衣"),
        )
        .expect_err("non-object state must fail");

        assert_eq!(error, "角色当前状态不是 JSON 对象");
    }

    #[test]
    fn attribute_state_projection_rejects_non_object_category() {
        let mut state = json!({
            "explicit_attributes": [],
            "implicit_attributes": {},
            "social_attributes": {}
        });

        let error = apply_attribute_to_state(
            &mut state,
            "explicit",
            "appearance",
            "modify",
            json!("白衣"),
        )
        .expect_err("non-object category must fail");

        assert_eq!(error, "角色当前状态 explicit_attributes 不是 JSON 对象");
    }

    #[test]
    fn foreshadowing_status_accepts_only_canonical_values() {
        assert_eq!(
            ForeshadowingStatus::parse("partially_revealed"),
            Some(ForeshadowingStatus::PartiallyRevealed)
        );
        assert!(ForeshadowingStatus::Pending.can_resolve());
        assert!(!ForeshadowingStatus::Resolved.can_resolve());
        assert_eq!(ForeshadowingStatus::parse(" pending "), None);
        assert_eq!(ForeshadowingStatus::parse("legacy_pending"), None);
    }
}
