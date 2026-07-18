pub(in crate::actions::video_records) fn validate_assembly_workflow_mode(
    value: &str,
) -> Result<&'static str, String> {
    match value.trim() {
        "boundary_frames" => Ok("boundary_frames"),
        "first_frame_chain" => Ok("first_frame_chain"),
        "reference_media" => Ok("reference_media"),
        "chat_video" => Ok("chat_video"),
        "" => Err("请选择视频生成流程后再拼接成片".to_string()),
        other => Err(format!("视频生成流程无效：{other}")),
    }
}
