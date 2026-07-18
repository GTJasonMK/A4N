#[test]
fn provider_prompt_rejects_stale_missing_input_image_status_with_bound_media() {
    let mut row = prompt_with_metadata();
    row.workflow_mode = "first_frame_chain".to_string();
    row.shot_number = 2;
    row.visual_prompt = "[主体定义与素材绑定]\n无输入首帧。本段仅依赖文字描述保持角色形象一致。\n\n[片段内容以及前因后果]\n角色承接上一段尾态继续阅读，读到关键句后抬头看向窗外。\n\n[镜头分镜]\n镜头 1：窗边中近景保持角色双手持信纸低头默读，下午暖光从窗侧落在脸部和桌面。\n镜头 2：镜头轻微推近，角色读到关键句后抬头看向窗外，手中信纸不离开画面。\n镜头 3：画面稳定在角色震惊与好奇交织的表情，视线锁定窗外并保留窗边光源方向。\n\n[画面风格描述]\n京阿尼动画风格，下午暖光，窗边中近景。\n\n[连续性与结尾状态]\n角色双手仍持信纸，视线锁定窗外，光源方向保持一致。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸或空间跳变。".to_string();

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(31),
            last: None,
        },
    )
    .expect_err("provider prompt must reject stale upload status text");

    assert!(err.contains("visual_prompt 包含上传状态说明"));
    assert!(err.contains("无输入首帧"));
}

#[test]
fn provider_prompt_rejects_stale_missing_input_image_status_without_bound_media() {
    let mut row = prompt_with_metadata();
    row.reference_image_ids_json = None;
    row.visual_prompt = "[主体定义与素材绑定]\n无输入首帧。本段仅依赖文字描述保持角色形象一致。\n\n[片段内容以及前因后果]\n角色承接上一段尾态继续阅读，读到关键句后抬头看向窗外。\n\n[镜头分镜]\n镜头 1：窗边中近景保持角色双手持信纸低头默读，下午暖光从窗侧落在脸部和桌面。\n镜头 2：镜头轻微推近，角色读到关键句后抬头看向窗外，手中信纸不离开画面。\n镜头 3：画面稳定在角色震惊与好奇交织的表情，视线锁定窗外并保留窗边光源方向。\n\n[画面风格描述]\n京阿尼动画风格，下午暖光，窗边中近景。\n\n[连续性与结尾状态]\n角色双手仍持信纸，视线锁定窗外，光源方向保持一致。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸或空间跳变。".to_string();

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: None,
            last: None,
        },
    )
    .expect_err("provider prompt must reject stale upload status text");

    assert!(err.contains("visual_prompt 包含上传状态说明"));
    assert!(err.contains("无输入首帧"));
}

#[test]
fn provider_prompt_rejects_stale_tail_frame_upload_status() {
    let mut row = prompt_with_metadata();
    row.visual_prompt = "[主体定义与素材绑定]\n首帧输入图片定义角色站在窗边的开头画面；没有输入尾帧，本段结尾按文字描述生成。\n\n[片段内容以及前因后果]\n角色承接上一段尾态继续阅读，读到关键句后抬头看向窗外，表情从专注转为震惊与好奇。\n\n[镜头分镜]\n镜头 1：窗边中近景承接首帧站位，角色双手持信纸低头默读，柔和自然光从左侧窗户照入。\n镜头 2：镜头轻微推近，角色读到关键句后抬头看向窗外，发丝和校服细节保持稳定。\n镜头 3：画面稳定在角色震惊与好奇交织的表情，视线锁定窗外，桌椅和窗光方向不变。\n\n[画面风格描述]\n京阿尼动画风格，下午暖光，窗边中近景，柔和自然光从左侧窗户照入。\n\n[连续性与结尾状态]\n角色双手仍持信纸，视线锁定窗外，发型、校服、桌椅和光源方向保持一致。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸或空间跳变。".to_string();

    let err = build_video_generation_provider_prompt(
        &row,
        &BoundaryFrameImageIds {
            first: Some(21),
            last: None,
        },
    )
    .expect_err("provider prompt must reject stale tail-frame upload status text");

    assert!(err.contains("visual_prompt 包含上传状态说明"));
    assert!(err.contains("没有输入尾帧"));
}
