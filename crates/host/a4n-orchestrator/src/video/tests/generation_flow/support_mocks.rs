use super::*;
use a4n_test_support::llm::CancellingLlmProvider;

use crate::video::prompt_context::video_source_metadata;
use std::{collections::HashMap, sync::OnceLock, time::Duration};

fn segment_plan_fixture() -> Value {
    json!({
        "segments": [
            {
                "source_shot_id": "shot-1",
                "covered_beat_ids": ["beat-1", "beat-2"],
                "title": "入殿",
                "scene_summary": "小宦官推门入殿，发现异常",
                "blocking_summary": "小宦官从殿门外进入殿内，门是固定空间锚点，人物始终在画面中央偏左",
                "spatial_anchors": [{"anchor": "殿门", "usage": "约束入殿起点和移动方向"}],
                "emotion_action_goals": [{"character": "小宦官", "emotion": "紧张", "visible_action": "放慢脚步并收紧肩膀"}],
                "video_intent": {"hook": "推门入殿后察觉异常", "audience_effect": "让观众感到紧张和不安", "rhythm": "缓慢进入后短暂停顿", "visual_priority": "小宦官推门、入殿、发现异常的动作"},
                "emotional_arc": {"start": "谨慎", "turn": "察觉殿内异常", "end": "紧张压住恐惧", "function": "场景建立"},
                "performance_plan": test_segment_performance_plan(
                    4,
                    "从殿门外中央偏左推门进入殿内，在暗廊入口前停住。",
                    "小宦官推门入殿，放慢脚步后停住观察。",
                    "先看殿门内侧，再落向右后方暗廊",
                    "平视中景缓慢推进，保持殿门和暗廊同框。",
                ),
                "material_coverage": {"required_entities": ["小宦官", "殿门", "殿内空间"], "missing_or_weak_references": ["小宦官", "殿门", "殿内空间"], "risk_note": "需要用文字明确人物服饰、殿门和殿内空间"}
            },
            {
                "source_shot_id": "shot-2",
                "covered_beat_ids": ["beat-3", "beat-4"],
                "title": "回望",
                "scene_summary": "小宦官回望暗处，压住恐惧",
                "blocking_summary": "小宦官从殿内中景转向暗廊，暗廊位于画面右后方，人物不要跳到暗廊另一侧",
                "spatial_anchors": [{"anchor": "暗廊入口", "usage": "约束回望方向和终点视线"}],
                "emotion_action_goals": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "停步、回头、轻微后缩"}],
                "video_intent": {"hook": "小宦官回望暗处压住恐惧", "audience_effect": "制造悬疑和压迫", "rhythm": "停顿后缓慢回望", "visual_priority": "回头、后缩和暗廊方向"},
                "emotional_arc": {"start": "紧张", "turn": "听见或感到暗处异动", "end": "恐惧但克制", "function": "悬念保留"},
                "performance_plan": test_segment_performance_plan(
                    4,
                    "从殿内中央偏左转向右后方暗廊，轻微后缩后停在入口前。",
                    "小宦官停步回头看向暗廊，轻微后缩并压住恐惧。",
                    "从前方缓慢移向右后方暗廊入口",
                    "平视中景轻微跟随回头，保持人物不越过暗廊入口。",
                ),
                "material_coverage": {"required_entities": ["小宦官", "暗廊入口"], "missing_or_weak_references": ["小宦官", "暗廊入口"], "risk_note": "需要用文字锁定暗廊在画面右后方"}
            }
        ]
    })
}

const FRAME_PROMPTS_JSON: &str = r#"{
  "frames": [
    {
      "frame_number": 1,
      "title": "门外首帧",
      "scene_summary": "小宦官站在殿门外",
      "frame_prompt": "小宦官站在紧闭宫门前的走廊中央偏左，十二三岁、内侍服饰整齐，半侧身面向厚重朱红殿门，右手抬起准备推门，殿门铜钉、门槛和两侧廊柱作为固定参照物，烛光从画面右侧斜照，地面拖出柔和阴影，整体为紧张电影感写实风格。",
      "blocking_state": {
        "characters": [{"name": "小宦官", "position": "殿门外中央偏左", "facing": "面向殿门", "pose": "抬手准备推门"}],
        "spatial_anchors": ["殿门"],
        "do_not_change": ["殿门始终在人物前方"]
      },
      "lighting_state": {"time_of_day": "夜晚", "key_light": "烛光", "direction": "画面右侧", "shadow": "柔和昏暗阴影", "intent": "紧张"},
      "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余肢体、面部不一致、空间关系错误",
      "duration_secs": 4,
      "aspect_ratio": "16:9",
      "reference_image_ids": [],
      "segment_numbers": [1]
    },
    {
      "frame_number": 2,
      "title": "入殿尾帧",
      "scene_summary": "承接上一帧，小宦官进入殿内并察觉暗处",
      "frame_prompt": "承接上一帧的小宦官已经进入昏暗殿内并停在中央偏左位置，身体略微侧向右后方暗角，肩膀收紧，内侍服饰和稚嫩脸部保持一致，身后的殿门、右后方阴影角落和地面烛光反射作为固定参照物，主光仍来自画面右侧烛火，画面保持悬疑写实和稳定中远景构图。",
      "blocking_state": {
        "characters": [{"name": "小宦官", "position": "殿内中央偏左", "facing": "看向右后方暗角", "pose": "停步侧身"}],
        "spatial_anchors": ["殿门", "暗角"],
        "do_not_change": ["暗角在画面右后方"]
      },
      "lighting_state": {"time_of_day": "夜晚", "key_light": "烛光", "direction": "画面右侧", "shadow": "殿内深阴影", "intent": "悬疑"},
      "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余肢体、面部不一致、光影不连续",
      "duration_secs": 4,
      "aspect_ratio": "16:9",
      "reference_image_ids": [],
      "segment_numbers": [1]
    },
    {
      "frame_number": 3,
      "title": "回望尾帧",
      "scene_summary": "承接上一帧，小宦官回望暗廊",
      "frame_prompt": "承接上一帧的小宦官靠近暗廊入口但仍停在殿内中央偏左，回头望向画面右后方黑暗走廊，肩膀收紧、脚步未越过入口，内侍服饰、脸部年龄感和站位保持连续，暗廊门洞、殿内柱影和摇动烛光作为固定参照物，右侧暖烛光制造拉长阴影，整体为压抑电影感写实风格。",
      "blocking_state": {
        "characters": [{"name": "小宦官", "position": "殿内中央偏左靠近暗廊入口", "facing": "回望右后方暗廊", "pose": "肩膀收紧"}],
        "spatial_anchors": ["暗廊入口"],
        "do_not_change": ["人物不越过暗廊入口"]
      },
      "lighting_state": {"time_of_day": "夜晚", "key_light": "烛光", "direction": "画面右侧", "shadow": "拉长的昏暗阴影", "intent": "恐惧"},
      "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余肢体、面部不一致、空间跳变",
      "duration_secs": 4,
      "aspect_ratio": "16:9",
      "reference_image_ids": [],
      "segment_numbers": [2]
    }
  ]
}"#;
