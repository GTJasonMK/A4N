pub fn is_system_managed_image_type(image_type: &str) -> bool {
    matches!(
        image_type.trim(),
        "scene"
            | "manga_page"
            | "scene_material"
            | "object_material"
            | "video_key_frame"
            | "video_start_frame"
            | "video_storyboard_frame"
            | "video_end_frame"
            | "video_tail_frame"
            | "canvas_image"
            | "canvas_frame"
            | "portrait"
            | "portrait_reference"
            | "chapter_reference"
            | "project_reference"
    )
}

#[cfg(test)]
mod tests {
    use super::is_system_managed_image_type;

    #[test]
    fn system_managed_image_types_have_one_canonical_catalog() {
        assert!(is_system_managed_image_type(" project_reference "));
        assert!(is_system_managed_image_type("video_tail_frame"));
        assert!(!is_system_managed_image_type("custom_reference"));
    }
}
