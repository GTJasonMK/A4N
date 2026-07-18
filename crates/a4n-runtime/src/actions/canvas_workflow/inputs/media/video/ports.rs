use a4n_core::models::canvas::canvas_edge;

pub(super) fn video_image_input_port(edge: &canvas_edge::Model) -> Result<&str, String> {
    match edge.target_port.as_str() {
        "first_frame" | "end_frame" | "reference_image" => Ok(edge.target_port.as_str()),
        _ => Err(format!(
            "GenerateVideoNode 图片输入端口无效: target_port={}, role={}",
            edge.target_port, edge.role
        )),
    }
}

pub(super) fn set_single_image_input(
    slot: &mut Option<i64>,
    image_id: i64,
    label: &str,
) -> Result<(), String> {
    if let Some(existing) = *slot
        && existing != image_id
    {
        return Err(format!("{label}只允许一个来源"));
    }
    *slot = Some(image_id);
    Ok(())
}
