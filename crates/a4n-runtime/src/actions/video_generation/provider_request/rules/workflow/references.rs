use crate::actions::video_generation::BoundaryFrameImageIds;

pub(in crate::actions::video_generation) fn reference_ids_with_boundary_frame_images(
    frame_ids: &BoundaryFrameImageIds,
    prompt_reference_ids: &[i64],
) -> Vec<i64> {
    let mut ids = Vec::new();
    for id in [frame_ids.first, frame_ids.last]
        .into_iter()
        .flatten()
        .chain(prompt_reference_ids.iter().copied())
    {
        if id > 0 && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

pub(in crate::actions::video_generation) fn reference_ids_with_storyboard_image(
    storyboard_image_id: Option<i64>,
    prompt_reference_ids: &[i64],
) -> Vec<i64> {
    let mut ids = Vec::new();
    for id in storyboard_image_id
        .into_iter()
        .chain(prompt_reference_ids.iter().copied())
    {
        if id > 0 && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}
