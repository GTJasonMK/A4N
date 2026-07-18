fn video_shot_json(shot_number: i64) -> String {
    let (title, scene_summary, visual_prompt, motion_prompt, dialogue_cues) = match shot_number {
        1 => (
            "入殿",
            "小宦官推门入殿，发现异常",
            "[主体定义与素材绑定]\n参考图用于保持小宦官的脸部、内侍服饰、朱红宫门、门槛、廊柱和右侧烛光方向一致。小宦官十二三岁，穿整齐内侍服饰，起点在宫门前画面中央偏左。\n\n[片段内容以及前因后果]\n小宦官夜里推开紧闭宫门进入殿内，刚进入就察觉右后方暗角有异常，情绪从谨慎转为紧张。\n\n[镜头分镜]\n镜头 1：中远景平视，朱红宫门、铜钉、门槛和两侧廊柱清楚可见，小宦官站在画面中央偏左，右手抬起触门。\n镜头 2：镜头缓慢向前推进，小宦官推门并放慢脚步进入殿内，肩膀逐渐收紧，殿门保持在身后。\n镜头 3：镜头稳定在殿内中央偏左，小宦官停步看向画面右后方暗角，烛光从画面右侧斜照。\n\n[画面风格描述]\n电影感写实，低饱和夜间宫殿光线，右侧暖烛光与殿内暗角形成冷暖对比，服饰褶皱和木门铜钉细节清晰，镜头语言克制。\n\n[连续性与结尾状态]\n结尾必须停在殿内中央偏左，小宦官看向右后方暗角，殿门在身后，烛光方向不变，可自然承接下一段回望暗廊。\n\n[限制与避免项]\n不要改变人物脸部、服装、宫门位置、暗角方向或光源方向。不要字幕、水印、空间跳变、跳轴、多余肢体或画面文字。",
            "[运镜与动作]\n镜头从门外中远景缓慢向前推进，小宦官先抬手触门，再推开宫门并放慢脚步进入殿内，肩膀逐渐收紧，动作速度克制而紧张。\n\n[对白与表演]\n本段无对白。表演重点是手指触门时的迟疑、进入殿内时的浅呼吸和停步看向暗角时的紧张眼神。",
            json!([]),
        ),
        2 => (
            "回望",
            "小宦官回望暗处，压住恐惧",
            "[主体定义与素材绑定]\n参考图用于保持小宦官的脸部、身形、内侍服饰、殿内柱影、暗廊入口和右侧烛光方向一致。小宦官承接上一段停在昏暗殿内中央偏左。\n\n[片段内容以及前因后果]\n小宦官察觉右后方暗处异动后停住，压低声音试探询问，并在无人回应的沉默里轻微后缩，恐惧被压在身体动作里。\n\n[镜头分镜]\n镜头 1：中景平视，小宦官停在殿内中央偏左，身体朝向右后方暗角，殿门余光和柱影作为固定参照。\n镜头 2：镜头保持轻微手持漂移但不跳轴，小宦官缓慢回头望向右后方暗廊，嘴唇微动说出短句。\n镜头 3：他原地轻微后缩，肩膀收紧，靠近暗廊入口但不越过，右侧暖烛光在地面拉出阴影。\n\n[画面风格描述]\n电影感写实，压抑悬疑的低照度宫殿室内，暖烛光来自画面右侧，柱影、暗廊入口和地面反光清楚但不喧宾夺主。\n\n[连续性与结尾状态]\n结尾人物靠近暗廊入口但仍停在入口前，回望黑暗走廊；暗廊始终位于画面右后方，光影方向与上一段连续。\n\n[限制与避免项]\n不要改变人物脸部、服装、暗廊方向、殿门余光、柱影或烛光方向。不要字幕、水印、空间跳变、跳轴、多余肢体或画面文字。",
            "[运镜与动作]\n镜头保持轻微手持漂移但不跳轴，小宦官先停在殿内中央偏左看向暗角，然后缓慢回头望向右后方暗廊，最后原地轻微后缩并收紧肩膀。\n\n[对白与表演]\n小宦官压低声音试探说“谁在那儿”，嘴唇短促微动，随后屏住呼吸；暗廊没有回应，他的肩膀更紧，眼神不敢完全移开黑暗处。",
            json!([{"speaker": "小宦官", "line": "谁在那儿", "delivery": "压低声音试探", "visible_action": "嘴唇微动并屏住呼吸", "listener_reaction": "暗廊没有回应，人物肩膀更紧", "timing": "中段"}]),
        ),
        other => panic!("unexpected video shot number: {other}"),
    };
    let shot = json!({
        "title": title,
        "scene_summary": scene_summary,
        "visual_prompt": visual_prompt,
        "motion_prompt": motion_prompt,
        "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余肢体、面部不一致、空间跳变、光影不连续",
        "dialogue_cues": dialogue_cues
    });
    json!({ "shots": [shot] }).to_string()
}

fn english_video_shot_json(shot_number: i64) -> String {
    let (title, scene_summary, focus) = match shot_number {
        1 => (
            "Entering The Hall",
            "The young eunuch pushes open the palace door and senses something wrong.",
            "the young eunuch steps through the vermilion palace door",
        ),
        2 => (
            "Looking Back",
            "The young eunuch looks back toward the dark corridor and suppresses fear.",
            "the young eunuch turns toward the dark corridor",
        ),
        other => panic!("unexpected English video shot number: {other}"),
    };
    let visual_prompt = format!(
        "[Subject And Asset Binding]\nThe segment begins from the established first-frame chain state; use the saved first-frame asset only as the opening visual anchor when present, while keeping the character identity, costume, palace door, threshold, pillars, and candle direction consistent.\n\n[Segment Context]\nThis shot shows {focus}. The character is a twelve or thirteen year old palace eunuch in neat attendant clothing, tense but controlled, moving through a dim palace interior where the right rear shadowed corner becomes the emotional threat.\n\n[Shot Storyboard]\nShot 1: an eye-level medium-wide view holds the starting pose, letting the character hesitate while the palace door, threshold, pillars, and candle direction remain readable.\nShot 2: the camera makes a restrained slow push as he advances carefully, shoulders tightening and steps staying small, with the door still behind him as a spatial anchor.\nShot 3: the movement settles into the required ending position; he pauses, looks toward the right rear darkness, and the frame stabilizes for downstream continuation.\n\n[Visual Style]\nCinematic realistic low-light palace lighting, warm candlelight from screen right, clean shadow edges, restrained camera language, detailed costume folds, muted colors, and a suspenseful but grounded atmosphere.\n\n[Continuity And End State]\nThe end frame must keep the character near the center-left palace interior position, facing the right rear dark corner, with the palace door behind him and the candlelight direction unchanged.\n\n[Constraints And Avoidances]\nDo not change the character face, costume, palace layout, camera axis, door position, candle direction, or spatial relationship between the character and the dark corner. Avoid blur, extra limbs, subtitles, watermarks, and sudden teleporting."
    );
    let motion_prompt = format!(
        "[Camera And Action]\nUse a restrained eye-level medium-wide camera with a very slow push forward. The action follows {focus}: his hand and shoulder move first, then his feet advance in small cautious steps, and the body settles into a tense pause near the end.\n\n[Dialogue And Performance]\nNo spoken dialogue in this test shot. Performance is expressed through shallow breathing, tightened shoulders, a slight pause before moving, and a controlled glance toward the right rear darkness."
    );
    json!({
        "shots": [
            {
                "title": title,
                "scene_summary": scene_summary,
                "visual_prompt": visual_prompt,
                "motion_prompt": motion_prompt,
                "negative_prompt": "low quality, blurry image, distorted anatomy, extra limbs, inconsistent face, spatial jump, lighting discontinuity, subtitles, watermark",
                "dialogue_cues": []
            }
        ]
    })
    .to_string()
}

fn regenerated_video_shot_json(shot_number: i64) -> String {
    let mut value: serde_json::Value = serde_json::from_str(&video_shot_json(shot_number))
        .expect("test shot json should remain valid JSON");
    let shot = value
        .get_mut("shots")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|shots| shots.first_mut())
        .expect("test shot wrapper should contain one shot");
    let visual_prompt = shot["visual_prompt"]
        .as_str()
        .expect("test shot should contain visual_prompt");
    shot["visual_prompt"] = json!(visual_prompt.replace(
        "[片段内容以及前因后果]\n",
        &format!("[片段内容以及前因后果]\n重新生成后的镜头 {shot_number} 视觉提示词。\n"),
    ));
    let motion_prompt = shot["motion_prompt"]
        .as_str()
        .expect("test shot should contain motion_prompt");
    shot["motion_prompt"] = json!(motion_prompt.replace(
        "[运镜与动作]\n",
        &format!("[运镜与动作]\n重新生成后的镜头 {shot_number} 运动提示词。\n"),
    ));
    value.to_string()
}

fn regenerated_video_shot_replies() -> Vec<String> {
    vec![
        regenerated_video_shot_json(1),
        regenerated_video_shot_json(2),
    ]
}

fn chat_video_shot_json(shot_number: i64) -> String {
    let (title, scene_summary, prompt_body) = match shot_number {
        1 => (
            "入殿",
            "小宦官推门入殿，发现异常",
            "\
[主体定义与素材绑定]
首帧画面：小宦官站在紧闭宫门前的走廊中央偏左，十二三岁，内侍服饰整齐，半侧身面向厚重朱红殿门，右手抬起准备推门，右侧烛光斜照在门槛和两侧廊柱上。
尾帧画面：小宦官已经进入昏暗殿内并停在中央偏左位置，身体略微侧向右后方暗角，肩膀收紧，身后的殿门、右后方阴影角落和地面烛光反射保持清晰。
本段无额外上传参考图，按文字描述生成。

[片段内容以及前因后果]
承接章节开头，小宦官夜里来到紧闭朱红宫门前。他需要推门进入殿内，逐渐发现右后方暗角有异常动静。镜头以殿门、门槛、铜钉和两侧廊柱作为空间锚点，人物始终位于画面中央偏左，动作克制紧张。

[镜头分镜]
镜头 1：中远景承接小宦官站在紧闭宫门前走廊中央偏左的首帧，右手抬起触向朱红宫门，右侧烛光照出门槛和两侧廊柱。
镜头 2：镜头保持同一轴线轻微前推，小宦官试探性推开宫门，动作克制，门缝扩大时烛光仍从画面右侧斜照。
镜头 3：小宦官放慢脚步进入昏暗殿内，殿门逐渐留在身后，右后方暗角进入画面并成为新的威胁锚点。
镜头 4：小宦官逐渐停步并侧向右后方暗角，肩膀收紧，镜头减速稳定到中远景，身后的殿门、暗角和地面烛光反射保持清晰。

[画面风格描述]
电影感写实，低照度宫殿烛光，暖色主光与深阴影对比，构图稳定，悬疑压抑。

[连续性与结尾状态]
以尾帧状态作为下一段衔接基准：小宦官停在殿内中央偏左并看向右后方暗角，殿门、暗角和烛光方向保持清晰，下一段从这个尾态继续。

[限制与避免项]
不要换脸、换装、跳轴、改变暗角方位，不要让人物突然移动到暗廊另一侧，不要生成多余肢体或模糊画面，不要配背景音乐。",
        ),
        2 => (
            "回望",
            "小宦官回望暗处，压住恐惧",
            "\
[主体定义与素材绑定]
首帧画面：小宦官已经进入昏暗殿内并停在中央偏左位置，身体略微侧向右后方暗角，肩膀收紧，内侍服饰和稚嫩脸部保持一致，右侧烛光照出殿门和阴影角落。
尾帧画面：小宦官靠近暗廊入口但仍停在殿内中央偏左，回头望向画面右后方黑暗走廊，肩膀收紧、脚步未越过入口，暗廊门洞和摇动烛光作为固定参照物。
本段无额外上传参考图，按文字描述生成。

[片段内容以及前因后果]
承接上一段结尾：小宦官已经停在昏暗殿内中央偏左并看向右后方暗角。本段表现他压住恐惧，缓慢回头望向暗廊入口，确认黑暗里是否有人。镜头保持同一空间轴线，暗廊始终在画面右后方。

[镜头分镜]
镜头 1：中景承接上一段尾态，小宦官停在殿内中央偏左，身体朝向右后方暗角，右侧烛光照出殿门和阴影角落。
镜头 2：镜头保持轻微手持漂移，小宦官屏住呼吸并原地轻微后缩，脚步没有越过暗廊入口。
镜头 3：小宦官慢慢回头望向右后方暗廊入口，肩膀收紧，嘴唇微动像是在试探，暗廊始终压在画面右后方。
镜头 4：人物靠近暗廊入口但不越过，镜头保持压抑悬疑状态并逐渐稳定，最后停在回望黑暗走廊的尾帧状态。

[画面风格描述]
电影感写实，昏暗宫殿、右侧暖烛光、柱影和地面反光延续上一段，镜头克制不跳轴。

[连续性与结尾状态]
以尾帧状态作为下一段衔接基准：小宦官靠近暗廊入口但没有越过，暗廊仍在画面右后方，人物服饰、脸部年龄感和烛光方向与上一段一致。

[限制与避免项]
不要让人物穿越到暗廊另一侧，不要改变暗廊与殿门方向，不要换脸换装，不要出现空间跳变、光影断裂、画面模糊或多余肢体，不要配背景音乐。",
        ),
        other => panic!("unexpected chat video shot number: {other}"),
    };
    json!({
        "shots": [
            {
                "title": title,
                "scene_summary": scene_summary,
                "chat_prompt": prompt_body,
                "negative_prompt": "换脸、换装、跳轴、空间跳变、光影断裂、画面模糊、多余肢体、背景音乐"
            }
        ]
    })
    .to_string()
}

fn regenerated_chat_video_shot_json(shot_number: i64) -> String {
    let mut value: serde_json::Value = serde_json::from_str(&chat_video_shot_json(shot_number))
        .expect("test chat shot json should remain valid JSON");
    let shot = value
        .get_mut("shots")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|shots| shots.first_mut())
        .expect("test chat shot wrapper should contain one shot");
    let prompt = shot
        .get("chat_prompt")
        .and_then(serde_json::Value::as_str)
        .expect("test chat shot should contain chat_prompt");
    shot["chat_prompt"] = json!(prompt.replace(
        "[片段内容以及前因后果]\n",
        &format!("[片段内容以及前因后果]\n重新生成后的 Chat 视频提示词 {shot_number}。\n"),
    ));
    value.to_string()
}

fn serial_chat_video_shot_replies() -> Vec<String> {
    vec![chat_video_shot_json(1), chat_video_shot_json(2)]
}

fn serial_video_shot_replies() -> Vec<String> {
    vec![video_shot_json(1), video_shot_json(2)]
}
