use a4n_core::models::{asset::object_profile, scene::scene_profile};
use serde_json::Value;

pub(crate) fn scene_profile_image_prompt(
    profile: &scene_profile::Model,
    style: Option<&str>,
) -> String {
    let style = style
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("清晰写实的场景概念设定图，空间明确，适合作为视频场景参考");
    let seed = if profile.image_prompt_seed.trim().is_empty() {
        "根据场景档案生成可复用的地点与空间设定图"
    } else {
        profile.image_prompt_seed.trim()
    };
    let entry_points = compact_json_hint(&profile.entry_points_json, "未记录明确出入口");
    let rules = compact_json_hint(&profile.rules_json, "未记录特殊规则");
    let current_state = compact_json_hint(&profile.current_state_json, "未记录当前状态变化");
    format!(
        "主体：{name}\n素材定位：可复用的场景/地点设定图，只展示空间环境本身，不表现具体剧情动作。\n参考画面：{seed}\n场景基础：{base}\n视觉特征：{visual}\n空间布局：{layout}\n氛围与光线：{atmosphere}\n出入口与连接：{entry_points}\n场景规则：{rules}\n当前状态：{current_state}\n构图与比例：以空间关系、可行动线、入口出口、主体区域和关键环境物件为重点；不要让人物成为主体；背景不能空泛，适合作为后续视频参考素材。\n画面风格：{style}\n风格执行：如果画面风格中包含“风格模板”或“风格关键词”，必须保留其中的中文风格名和关键英文风格名，并体现对应光影、色彩、构图、镜头、材质、氛围、细节和避免项，不要只概括成统一风格。\n避免项：不要文字、不要水印、不要边框、不要纯抽象氛围图、不要把场景画成单个物体特写。",
        name = profile.name,
        base = profile.base_description,
        visual = profile.visual_description,
        layout = profile.spatial_layout,
        atmosphere = profile.atmosphere
    )
}

pub(crate) fn object_profile_image_prompt(
    profile: &object_profile::Model,
    style: Option<&str>,
) -> String {
    let style = style
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("清晰写实的概念设定图，主体明确，背景简洁");
    let location = profile
        .location_context
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("不强调具体地点");
    let seed = if profile.image_prompt_seed.trim().is_empty() {
        "根据外观细节生成清晰的单物体素材图"
    } else {
        profile.image_prompt_seed.trim()
    };
    format!(
        "主体：{name}\n素材定位：单个物体的静物道具素材图，只展示物体外观，不表现使用过程。\n参考画面：{seed}\n外观细节：{visual}\n材质与当前状态：{material}\n环境语境：{location}\n构图与比例：以单个物体为画面中心，完整展示轮廓和关键细节，背景简洁，不出现人物持握、攻击、伤害或威胁行为，适合作为后续视频参考素材。\n画面风格：{style}\n风格执行：如果画面风格中包含“风格模板”或“风格关键词”，必须保留其中的中文风格名和关键英文风格名，并体现对应光影、色彩、构图、镜头、材质、氛围、细节和避免项，不要只概括成统一风格。\n避免项：不要文字、不要水印、不要边框、不要多余人物、不要把它画成其他物体。",
        name = profile.name,
        visual = profile.visual_description,
        material = profile.material_and_state
    )
}

pub(crate) fn scene_profile_content(profile: &scene_profile::Model) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        profile.name,
        profile.base_description,
        profile.visual_description,
        profile.spatial_layout,
        profile.atmosphere,
        profile.entry_points_json,
        profile.rules_json,
        profile.image_prompt_seed
    )
}
pub(crate) fn object_profile_content(profile: &object_profile::Model) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        profile.name,
        profile.visual_description,
        profile.material_and_state,
        profile.story_usage,
        profile.visual_usage,
        profile.generation_usage,
        profile.location_context.as_deref().unwrap_or(""),
        profile.image_prompt_seed
    )
}

pub(super) fn metadata_text<'a>(metadata: Option<&'a Value>, key: &str) -> Option<&'a str> {
    metadata
        .and_then(|m| m.get(key))
        .and_then(Value::as_str)
        .and_then(non_empty)
}
pub(super) fn first_non_empty(values: &[&str]) -> String {
    values
        .iter()
        .map(|v| v.trim())
        .find(|v| !v.is_empty())
        .unwrap_or("")
        .to_string()
}
pub(super) fn first_non_empty_opt(values: &[Option<&str>]) -> String {
    values
        .iter()
        .flatten()
        .map(|v| v.trim())
        .find(|v| !v.is_empty())
        .unwrap_or("")
        .to_string()
}
pub(super) fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() { None } else { Some(value) }
}
pub(super) fn compact_json_hint(value: &Value, empty_text: &str) -> String {
    match value {
        Value::Null => empty_text.to_string(),
        Value::Array(v) if v.is_empty() => empty_text.to_string(),
        Value::Object(v) if v.is_empty() => empty_text.to_string(),
        _ => serde_json::to_string(value).expect("serde_json::Value serialization must succeed"),
    }
}
