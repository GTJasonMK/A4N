use super::*;

#[test]
fn frame_prompt_style_template_uses_prompt_output_language_sections() {
    let prompt = "[Reference Constraints]None.\n[Main Subject]The protagonist sits on the warehouse floor.\n[Space And Props]Broken bricks and shallow puddles stay fixed.\n[Composition Lighting Style]Cold side backlight.\n[Continuity And Avoidances]Keep character identity and warehouse anchors consistent.";
    let with_style = frame_prompt_with_style_template(
        prompt,
        "KyoAni inspired anime style",
        PromptOutputLanguage::En,
    );

    assert!(with_style.contains("[Composition Lighting Style]"));
    assert!(with_style.contains("Verbatim visual style template: KyoAni inspired anime style"));
    assert!(with_style.contains("[Continuity And Avoidances]"));
    assert!(!with_style.contains("【构图光线风格】"));
}

#[test]
fn boundary_frame_validation_accepts_safe_rewrite_prompt() {
    let mut frame = boundary_frame(1, Vec::new(), vec![1]);
    frame.frame_prompt =
        "【参考图约束】无。【画面主体】林寻坐在仓库地面，手停在安全距离外，表情压抑决绝。【空间与道具】危险铁片落在地面远处，碎砖和墙角固定不变。【构图光线风格】冷色中景，侧光从左前方进入。【一致性与避免项】保持人物、服装、仓库锚点和光源方向，避免血腥喷溅。"
            .to_string();

    validate_single_boundary_frame_prompt(&frame, &[segment(1)], &HashSet::new(), None)
        .expect("safe rewrite prompt should pass");
}

#[test]
fn boundary_frame_validation_leaves_unsafe_wording_to_prompt_optimization() {
    let mut frame = boundary_frame(1, Vec::new(), vec![1]);
    frame.frame_prompt =
        "【参考图约束】无。【画面主体】林寻把铁片刺入眼睛的执行瞬间，表情痛苦。【空间与道具】铁片贴近面部。【构图光线风格】冷色近景。【一致性与避免项】保持人物一致。"
            .to_string();

    validate_single_boundary_frame_prompt(&frame, &[segment(1)], &HashSet::new(), None)
        .expect("unsafe wording is left to prompt optimization instead of hard validation");
}

#[test]
fn boundary_frame_validation_rejects_internal_reference_id_text() {
    let mut frame = boundary_frame(1, vec![3], vec![1]);
    frame.frame_prompt =
        "【参考图约束】参考图 #3 约束林寻外观和仓库空间。【画面主体】林寻坐在地面，表情警觉。【空间与道具】水洼、碎砖和货架保持固定。【构图光线风格】冷色中景，侧逆光从右后方进入。【一致性与避免项】保持人物服装、空间锚点和光源方向。"
            .to_string();
    let err =
        validate_single_boundary_frame_prompt(&frame, &[segment(1)], &HashSet::from([3]), None)
            .expect_err("internal reference image id text should fail visibly");

    assert!(err.to_string().contains("内部图片 ID"));
}

#[test]
fn boundary_frame_validation_requires_previous_image_as_first_reference() {
    let segment_plan = vec![segment(1), segment(2)];
    let mut frame = boundary_frame(2, vec![3, 13], vec![1, 2]);
    frame.frame_prompt =
        "【参考图约束】参考图 1 约束人物外观，参考图 2 约束上一帧连续性。【画面主体】林寻站在仓库门边，表情警觉。【空间与道具】水洼、碎砖和货架保持固定。【构图光线风格】冷色中景，侧逆光从右后方进入。【一致性与避免项】保持人物服装、空间锚点和光源方向。"
            .to_string();

    let err = validate_single_boundary_frame_prompt(
        &frame,
        &segment_plan,
        &HashSet::from([3, 13]),
        Some(13),
    )
    .expect_err("previous frame image must be reference image 1");

    assert!(err.to_string().contains("reference_image_ids[0]"));
}

#[test]
fn validates_reused_boundary_frames_with_previous_image_allowed() {
    let segment_plan = vec![segment(1), segment(2)];
    let frames = vec![
        boundary_frame(1, vec![3, 9], vec![1]),
        boundary_frame(2, vec![13, 3, 9], vec![1, 2]),
        boundary_frame(3, vec![3, 9], vec![2]),
    ];
    let base_allowed = HashSet::from([3, 9]);
    let mut allowed_with_previous_image = base_allowed.clone();
    allowed_with_previous_image.insert(13);

    let err = validate_frame_prompts(&frames, &segment_plan, &base_allowed)
        .expect_err("previous boundary frame image must be explicitly allowed");
    assert!(err.to_string().contains("图片 ID：13"));
    validate_frame_prompts(&frames, &segment_plan, &allowed_with_previous_image)
        .expect("previous boundary frame image should be accepted as an available reference");
}
