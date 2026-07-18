fn valid_segments() -> Vec<VideoSegmentPlan> {
    vec![serde_json::from_value(json!({
        "schema_version": a4n_core::video_execution::VIDEO_SEGMENT_PLAN_V2,
        "segment_number": 1,
        "source_shot_id": "scene-1-shot-1",
        "segment_index_in_shot": 1,
        "shot_segment_count": 1,
        "covered_beat_ids": ["scene-1-shot-1-beat-1", "scene-1-shot-1-beat-2"],
        "title": "开场",
        "scene_summary": "主角进入档案室",
        "blocking_summary": "主角从档案室门口走向书桌",
        "spatial_anchors": [{"anchor": "书桌", "usage": "约束主角终点"}],
        "emotion_action_goals": [{
            "character": "主角",
            "emotion": "谨慎",
            "visible_action": "放慢脚步并观察"
        }],
        "video_intent": {
            "hook": "主角第一次进入异常档案室",
            "audience_effect": "让观众感到安静压迫和不确定",
            "rhythm": "缓慢推进",
            "visual_priority": "主角从门口到书桌的谨慎移动"
        },
        "emotional_arc": {
            "start": "谨慎观察",
            "turn": "发现档案室气氛异常",
            "end": "停在书桌旁保持戒备",
            "function": "场景建立"
        },
        "performance_plan": {
            "movement_path": "从档案室门口偏左向画面深处书桌慢走，沿左侧书架保持半步距离。",
            "body_performance": "肩膀略收紧，步幅放慢，靠近书桌时重心前移后停住。",
            "gaze_and_hands": "视线先扫过书架阴影再落到书桌，右手轻碰外套边缘。",
            "pause_or_reaction": "靠近书桌前停半拍，确认异常后不急着触碰物件。",
            "camera_relationship": "镜头平视缓慢推进，始终保留门口、书架和书桌的距离关系。",
            "time_blocks": [
                {
                    "start_sec": 0,
                    "end_sec": 3,
                    "camera": "门口宽中景，门框和书桌同框。",
                    "subject_action": "主角停在门口偏左，先观察书架和书桌。",
                    "continuity_target": "左侧窗光和门口位置保持不变。"
                },
                {
                    "start_sec": 3,
                    "end_sec": 6,
                    "camera": "缓慢推进到中景。",
                    "subject_action": "主角沿书架左侧走向书桌，在书桌旁停住。",
                    "continuity_target": "结尾保留主角、书桌和书架的固定关系。"
                }
            ],
            "action_script": [
                {
                    "character": "主角",
                    "body": "肩膀收紧，脚步放慢。",
                    "movement": "从画面左前门口走向画面深处中央书桌。",
                    "hands": "右手轻碰外套边缘，靠近书桌时停住。",
                    "gaze": "从书架阴影转向书桌表面。",
                    "timing": "全段"
                }
            ]
        },
        "material_coverage": {
            "required_entities": ["主角", "档案大厅"],
            "missing_or_weak_references": [],
            "risk_note": "关键人物和场景已有参考图"
        },
        "transition_to_next": null,
        "source_span_ids": ["ch1:s1:f1"],
        "duration_secs": 6,
        "aspect_ratio": "16:9",
        "reference_image_ids": [1, 2]
    }))
    .expect("typed video segment fixture should deserialize")]
}

fn segment_map(segments: &[VideoSegmentPlan]) -> HashMap<i32, &VideoSegmentPlan> {
    segments
        .iter()
        .map(|segment| (segment.segment_number, segment))
        .collect()
}

fn valid_shot_value(_shot_number: i32) -> Value {
    json!({
        "title": "开场",
        "scene_summary": "主角进入档案室",
        "visual_prompt": "[主体定义与素材绑定]\n参考图 1 用于主角脸部、发型、年龄感和深色长外套一致性；参考图 2 用于档案大厅入口、木质书架、铜制吊灯和中央书桌的空间基准。\n\n[片段内容以及前因后果]\n主角从档案室门口偏左的起点进入，目标是靠近深处书桌并确认异常气氛，结尾交给下一段的状态是他停在书桌旁保持戒备。\n\n[镜头分镜]\n镜头 1：门口宽中景保持门框、书架和书桌同框，主角站在画面左侧先停半拍观察，视线从书架阴影扫到深处书桌，空间保持安静压迫。\n镜头 2：平视中景缓慢推进，主角沿左侧书架向书桌慢走，肩膀略收紧，右手轻碰外套边缘，最后在书桌旁停住并保持戒备。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照，阴影落向右后方，构图稳定并突出门、书架和书桌的空间关系。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变，结尾停在书桌旁，表情戒备，左侧窗光方向保持连续。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变、镜头过快或角色服装变化。",
        "motion_prompt": "[运镜与动作]\n镜头从门口宽景缓慢向前推进，只保留一个稳定推进运动，动作节奏严格承接 visual_prompt 中的镜头分镜。\n\n[对白与表演]\n主角低声短句“这里不对劲”，嘴唇微动，脚步放慢。",
        "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余肢体、文字不可读、面部不一致、空间跳变、光影不连续",
        "dialogue_cues": [
            {"speaker": "主角", "line": "这里不对劲", "delivery": "低声迟疑", "visible_action": "嘴唇微动并放慢脚步", "listener_reaction": "无人回应，空间保持寂静", "timing": "中段"}
        ]
    })
}

fn response_from_shots(shots: Vec<Value>) -> String {
    json!({ "shots": shots }).to_string()
}

fn parse_with_valid_segments(raw: &str) -> Result<Vec<VideoPromptShot>, AppError> {
    let segments = valid_segments();
    parse_video_prompt_shots(raw, &segment_map(&segments), PromptOutputLanguage::ZhCn)
}

fn valid_chat_prompt_text() -> String {
    [
        "[主体定义与素材绑定]\n画面从档案室门口开始，主角站在门口偏左，暖色窗光从左侧斜照，远处书桌和书架作为空间锚点。",
        "[片段内容以及前因后果]\n主角进入档案室并保持谨慎。",
        "[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。",
        "[画面风格描述]\n电影感写实，暖色窗光。",
        "[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。",
        "[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。",
    ]
    .join("\n")
}

fn valid_chat_video_shot_value(_shot_number: i32) -> Value {
    json!({
        "title": "开场",
        "scene_summary": "主角进入档案室",
        "chat_prompt": valid_chat_prompt_text(),
        "negative_prompt": "避免字幕、水印、换脸、空间跳变"
    })
}

fn parse_chat_with_valid_segments(raw: &str) -> Result<Vec<ChatVideoPromptShot>, AppError> {
    let segments = valid_segments();
    parse_chat_video_prompt_shots(raw, &segment_map(&segments), PromptOutputLanguage::ZhCn)
}
