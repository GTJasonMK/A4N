use a4n_core::{
    models::chapter::chapter_shooting_script, shooting_script::ChapterShootingScriptV1,
};

pub use a4n_api_contract::shooting_script::{
    ChapterShootingScriptView, EditChapterShootingScriptRecordInput,
    EditedChapterShootingScriptView, GenerateChapterShootingScriptInput,
    GetChapterShootingScriptInput,
};

pub(super) fn contract_view(
    record: chapter_shooting_script::Model,
    script: ChapterShootingScriptV1,
) -> Result<ChapterShootingScriptView, String> {
    Ok(ChapterShootingScriptView {
        record: serde_json::to_value(record)
            .map_err(|error| format!("序列化拍摄剧本记录失败: {error}"))?,
        script: serde_json::to_value(script)
            .map_err(|error| format!("序列化拍摄剧本合同失败: {error}"))?,
    })
}

pub(super) fn domain_script(script: serde_json::Value) -> Result<ChapterShootingScriptV1, String> {
    serde_json::from_value(script).map_err(|error| format!("解析拍摄剧本合同失败: {error}"))
}
