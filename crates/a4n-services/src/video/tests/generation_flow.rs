use super::*;
use crate::{source::source_content_sha256, video::prompt_context::video_source_metadata};
use futures::stream;
use std::{
    collections::{HashMap, VecDeque},
    future,
    pin::Pin,
    sync::{Mutex, OnceLock},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

const SEGMENT_PLAN_JSON: &str = r#"{
  "segments": [
    {
      "analysis_shot_id": 1,
      "covered_beat_ids": [1, 2],
      "title": "入殿",
      "scene_summary": "小宦官推门入殿，发现异常",
      "blocking_summary": "小宦官从殿门外进入殿内，门是固定空间锚点，人物始终在画面中央偏左",
      "spatial_anchors": [{"anchor": "殿门", "usage": "约束入殿起点和移动方向"}],
      "emotion_action_goals": [{"character": "小宦官", "emotion": "紧张", "visible_action": "放慢脚步并收紧肩膀"}],
      "video_intent": {"hook": "推门入殿后察觉异常", "audience_effect": "让观众感到紧张和不安", "rhythm": "缓慢进入后短暂停顿", "visual_priority": "小宦官推门、入殿、发现异常的动作"},
      "emotional_arc": {"start": "谨慎", "turn": "察觉殿内异常", "end": "紧张压住恐惧", "function": "场景建立"},
      "material_coverage": {"required_entities": ["小宦官", "殿门", "殿内空间"], "missing_or_weak_references": ["小宦官", "殿门", "殿内空间"], "risk_note": "需要用文字明确人物服饰、殿门和殿内空间"},
      "duration_secs": 4
    },
    {
      "analysis_shot_id": 2,
      "covered_beat_ids": [1, 2],
      "title": "回望",
      "scene_summary": "小宦官回望暗处，压住恐惧",
      "blocking_summary": "小宦官从殿内中景转向暗廊，暗廊位于画面右后方，人物不要跳到暗廊另一侧",
      "spatial_anchors": [{"anchor": "暗廊入口", "usage": "约束回望方向和终点视线"}],
      "emotion_action_goals": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "停步、回头、轻微后缩"}],
      "video_intent": {"hook": "小宦官回望暗处压住恐惧", "audience_effect": "制造悬疑和压迫", "rhythm": "停顿后缓慢回望", "visual_priority": "回头、后缩和暗廊方向"},
      "emotional_arc": {"start": "紧张", "turn": "听见或感到暗处异动", "end": "恐惧但克制", "function": "悬念保留"},
      "material_coverage": {"required_entities": ["小宦官", "暗廊入口"], "missing_or_weak_references": ["小宦官", "暗廊入口"], "risk_note": "需要用文字锁定暗廊在画面右后方"},
      "duration_secs": 4
    }
  ]
}"#;

const SHOT_ANALYSIS_JSON: &str = r#"{
  "cinematic_shots": [
    {
      "beat_ids": [1, 2],
      "title": "入殿",
      "scene_intent": "小宦官推门入殿并察觉异常",
      "visible_actions": [{"character": "小宦官", "action": "推门、入殿、停步观察", "purpose": "建立紧张情绪和宫殿空间"}],
      "dialogue_or_speech_intent": {"has_dialogue": false, "speaker": null, "key_line_or_intent": "无对白，依靠动作表现紧张", "visible_performance": "放慢脚步、肩膀收紧"},
      "blocking": {"start": "殿门外中央偏左", "movement": "推门进入殿内", "end": "殿内中央偏左", "spatial_anchors": ["殿门", "殿内空间"]},
      "camera_intent": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "缓慢推进", "visual_priority": "推门入殿和察觉异常"},
      "transition_in": "从殿门外准备推门开始",
      "transition_out": "停在殿内并看向右后方暗角",
      "start_state": {"blocking": "殿门外中央偏左", "emotion": "谨慎", "camera": "平视中景"},
      "end_state": {"blocking": "殿内中央偏左", "emotion": "紧张", "camera": "平视中景保持空间连续"},
      "action_load": "medium",
      "continuity_handoff": {"must_preserve": ["小宦官身份", "殿门和暗角方向"], "handoff_state": "小宦官停在殿内中央偏左并看向右后方暗角"}
    },
    {
      "beat_ids": [3, 4],
      "title": "回望",
      "scene_intent": "小宦官回望暗处并压住恐惧",
      "visible_actions": [{"character": "小宦官", "action": "停步、回头、轻微后缩", "purpose": "延续上一镜头的紧张并制造悬疑"}],
      "dialogue_or_speech_intent": {"has_dialogue": false, "speaker": null, "key_line_or_intent": "无对白，依靠回头和后缩表现恐惧", "visible_performance": "眼神回望、肩膀收紧"},
      "blocking": {"start": "殿内中央偏左", "movement": "转身回望右后方暗廊", "end": "靠近暗廊入口但不越界", "spatial_anchors": ["暗廊入口", "殿内柱影"]},
      "camera_intent": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "轻微跟随回头", "visual_priority": "回望动作和暗廊方向"},
      "transition_in": "承接上一镜头殿内中央偏左站位",
      "transition_out": "停在暗廊入口前保留悬念",
      "start_state": {"blocking": "殿内中央偏左", "emotion": "紧张", "camera": "平视中景"},
      "end_state": {"blocking": "暗廊入口前", "emotion": "恐惧但克制", "camera": "平视中景保持空间连续"},
      "action_load": "medium",
      "continuity_handoff": {"must_preserve": ["小宦官身份", "暗廊在画面右后方"], "handoff_state": "小宦官靠近暗廊入口但仍未越界"}
    }
  ]
}"#;

const BEAT_EXTRACTION_JSON: &str = r#"{
  "beats": [
    {
      "natural_seconds": 2.0,
      "required": true,
      "visible_action": "小宦官推门入殿并放慢脚步",
      "source_span_ids": ["ch1:s1:f1"]
    },
    {
      "natural_seconds": 2.0,
      "required": true,
      "visible_action": "小宦官停步观察并察觉暗处异常",
      "source_span_ids": ["ch1:s1:f2"]
    },
    {
      "natural_seconds": 2.0,
      "required": true,
      "visible_action": "小宦官停步回头看向右后方暗廊",
      "source_span_ids": ["ch1:s1:f2"]
    },
    {
      "natural_seconds": 2.0,
      "required": true,
      "visible_action": "小宦官轻微后缩并压住恐惧",
      "source_span_ids": ["ch1:s1:f3"]
    }
  ]
}"#;

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
      "title": "回望首帧",
      "scene_summary": "小宦官停在殿内并看向右后方暗角",
      "frame_prompt": "小宦官停在昏暗殿内中央偏左，身体略微侧向右后方暗角，肩膀收紧，内侍服饰和稚嫩脸部保持一致，身后的殿门、右后方阴影角落和地面烛光反射作为固定参照物，主光仍来自画面右侧烛火，画面保持悬疑写实和稳定中远景构图。",
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
      "segment_numbers": [2]
    },
    {
      "frame_number": 4,
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

struct CancellingLlmProvider {
    cancel_token: CancellationToken,
}

#[async_trait::async_trait]
impl LlmProvider for CancellingLlmProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, AppError> {
        self.cancel_token.cancel();
        future::pending::<Result<ChatResponse, AppError>>().await
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        Ok(true)
    }

    fn model_name(&self) -> &str {
        "cancelling-test-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            function_calling: false,
            vision: false,
            max_context_tokens: 8192,
            max_output_tokens: 4096,
        }
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.chars().count()
    }
}

struct CapturingSequenceLlmProvider {
    replies: Mutex<VecDeque<String>>,
    user_messages: Mutex<Vec<String>>,
}

impl CapturingSequenceLlmProvider {
    fn new_owned(replies: Vec<String>) -> Self {
        Self {
            replies: Mutex::new(replies.into_iter().collect()),
            user_messages: Mutex::new(Vec::new()),
        }
    }

    fn captured_user_messages(&self) -> Vec<String> {
        self.user_messages.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl LlmProvider for CapturingSequenceLlmProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        self.user_messages
            .lock()
            .unwrap()
            .extend(request.messages.iter().filter_map(|message| {
                matches!(&message.role, MessageRole::User).then(|| message.content.clone())
            }));
        let content = self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("测试 LLM 响应已耗尽")))?;
        Ok(ChatResponse {
            message: ChatMessage::assistant(content),
            usage: None,
            finish_reason: Some("stop".to_string()),
        })
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        Ok(true)
    }

    fn model_name(&self) -> &str {
        "capturing-sequence-test-llm"
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            function_calling: false,
            vision: false,
            max_context_tokens: 8192,
            max_output_tokens: 4096,
        }
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.chars().count()
    }
}

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
            "入殿到回望过渡",
            "小宦官从察觉暗角的停步尾态过渡到回望暗廊的起始姿态",
            "[主体定义与素材绑定]\n参考图用于保持小宦官的脸部、内侍服饰、殿内柱影、暗廊入口和右侧烛光方向一致。\n\n[片段内容以及前因后果]\n这是两个内容片段之间的过渡单元，只表现小宦官从停在殿内中央偏左、看向右后方暗角，到准备回望暗廊入口之间的动作桥。\n\n[镜头分镜]\n镜头 1：中景平视承接上一内容片段尾态，小宦官停在殿内中央偏左，肩膀收紧。\n镜头 2：镜头轻微稳定漂移，小宦官目光从暗角移向暗廊入口，身体重心稍微后撤。\n镜头 3：动作稳定到下一内容片段首态，他仍停在入口前，准备回望黑暗走廊。\n\n[画面风格描述]\n电影感写实，压抑悬疑的低照度宫殿室内，暖烛光来自画面右侧。\n\n[连续性与结尾状态]\n结尾稳定到回望暗廊的起始姿态，不新增关键剧情。\n\n[限制与避免项]\n不要重复推门入殿或完整回望询问动作，不要改变人物脸部、服装、暗廊方向、柱影或烛光方向。",
            "[运镜与动作]\n镜头保持同一空间轴线，小宦官只完成视线转移、重心后撤和肩膀收紧的过渡动作。\n\n[对白与表演]\n本过渡无对白，只保留屏息和迟疑。",
            json!([]),
        ),
        3 => (
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
        regenerated_video_shot_json(1),
        regenerated_video_shot_json(2),
        regenerated_video_shot_json(2),
        regenerated_video_shot_json(3),
        regenerated_video_shot_json(3),
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
            "入殿到回望过渡",
            "小宦官从入殿尾态过渡到回望暗廊的首态",
            "\
[主体定义与素材绑定]
首帧画面：小宦官已经进入昏暗殿内并停在中央偏左位置，身体略微侧向右后方暗角，肩膀收紧，身后的殿门、右后方阴影角落和地面烛光反射保持清晰。
尾帧画面：小宦官仍在殿内中央偏左，目光从暗角移向右后方暗廊入口，身体重心略微后撤，准备进入下一段回望动作。
本段无额外上传参考图，按文字描述生成。

[片段内容以及前因后果]
这是两个内容片段之间的过渡单元，只补足上一段尾态到下一段首态之间的动作桥。小宦官不再重复推门，也不完整执行回望询问，只完成视线转移、肩膀收紧和重心后撤。

[镜头分镜]
镜头 1：中景承接上一段尾帧，小宦官停在殿内中央偏左，身后殿门和右侧烛光方向保持不变。
镜头 2：镜头保持同一轴线轻微漂移，小宦官目光从右后方暗角移向暗廊入口，肩膀进一步收紧。
镜头 3：人物身体重心略微后撤但不跨越暗廊入口，画面稳定到下一内容片段的首帧状态。

[画面风格描述]
电影感写实，低照度宫殿烛光，暖色主光与深阴影对比，构图稳定，过渡动作克制。

[连续性与结尾状态]
结尾停在下一段可自然开始的首态：小宦官位于殿内中央偏左，准备回望右后方暗廊入口，殿门、暗角和烛光方向保持清晰。

[限制与避免项]
不要重复推门入殿，不要提前完成询问动作，不要换脸、换装、跳轴、改变暗角方位，不要生成多余肢体或模糊画面，不要配背景音乐。",
        ),
        3 => (
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
    vec![
        chat_video_shot_json(1),
        chat_video_shot_json(1),
        chat_video_shot_json(2),
        chat_video_shot_json(2),
        chat_video_shot_json(3),
        chat_video_shot_json(3),
    ]
}

fn serial_video_shot_replies() -> Vec<String> {
    vec![
        video_shot_json(1),
        video_shot_json(1),
        video_shot_json(2),
        video_shot_json(2),
        video_shot_json(3),
        video_shot_json(3),
    ]
}

fn frame_generation_options() -> ChapterVideoPromptOptions {
    ChapterVideoPromptOptions {
        style: "电影感写实".to_string(),
        prompt_output_language: None,
        prompt_batch_size: 12,
        duration_secs: 4,
        duration_secs_options: vec![4],
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: Vec::new(),
        video_library_image_ids: Vec::new(),
        duration_capability: None,
        regenerate_existing: false,
        workflow_run_id: None,
    }
}

fn video_generation_options_with_batch_size(prompt_batch_size: i32) -> ChapterVideoPromptOptions {
    let mut options = frame_generation_options();
    options.prompt_batch_size = prompt_batch_size;
    options
}

fn english_video_generation_options_with_batch_size(
    prompt_batch_size: i32,
) -> ChapterVideoPromptOptions {
    let mut options = video_generation_options_with_batch_size(prompt_batch_size);
    options.prompt_output_language = Some("en".to_string());
    options
}

fn regenerate_generation_options() -> ChapterVideoPromptOptions {
    let mut options = frame_generation_options();
    options.regenerate_existing = true;
    options
}

fn regenerate_generation_options_with_duration(duration_secs: i32) -> ChapterVideoPromptOptions {
    let mut options = regenerate_generation_options();
    options.duration_secs = duration_secs;
    options
}

fn single_segment_plan_json(duration_secs: i32) -> String {
    let mut value: serde_json::Value = serde_json::from_str(SEGMENT_PLAN_JSON).unwrap();
    value["segments"].as_array_mut().unwrap().truncate(1);
    value["segments"][0]["covered_beat_ids"] = json!([1, 2, 3, 4]);
    value["segments"][0]["duration_secs"] = json!(duration_secs);
    value.to_string()
}

fn shot_analysis_json_with_duration(duration_secs: i32) -> String {
    let mut value: serde_json::Value = serde_json::from_str(SHOT_ANALYSIS_JSON).unwrap();
    for shot in value["cinematic_shots"].as_array_mut().unwrap() {
        shot["duration_secs"] = json!(duration_secs);
    }
    value.to_string()
}

fn single_shot_analysis_json(duration_secs: i32) -> String {
    let mut value: serde_json::Value =
        serde_json::from_str(&shot_analysis_json_with_duration(duration_secs)).unwrap();
    value["cinematic_shots"].as_array_mut().unwrap().truncate(1);
    value.to_string()
}

fn segment_plan_json_for_shot(shot_id: i32, duration_secs: Option<i32>) -> String {
    let value: serde_json::Value = serde_json::from_str(SEGMENT_PLAN_JSON).unwrap();
    let segments = value["segments"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|segment| segment["analysis_shot_id"].as_i64() == Some(i64::from(shot_id)))
        .map(|segment| {
            let mut segment = segment.clone();
            if let Some(duration_secs) = duration_secs {
                segment["duration_secs"] = json!(duration_secs);
            }
            segment
        })
        .collect::<Vec<_>>();
    json!({ "segments": segments }).to_string()
}

fn frame_prompt_json(frame_number: usize) -> String {
    let value: serde_json::Value = serde_json::from_str(FRAME_PROMPTS_JSON).unwrap();
    let mut frame = value["frames"][frame_number - 1].clone();
    if let Some(frame) = frame.as_object_mut() {
        frame.remove("frame_number");
        frame.remove("duration_secs");
        frame.remove("aspect_ratio");
        frame.remove("segment_numbers");
    }
    json!({ "frames": [frame] }).to_string()
}

fn frame_prompt_json_with_references(frame_number: usize, reference_image_ids: &[i64]) -> String {
    let mut value: serde_json::Value =
        serde_json::from_str(&frame_prompt_json(frame_number)).unwrap();
    value["frames"][0]["reference_image_ids"] = json!(reference_image_ids);
    value.to_string()
}

fn frame_prompt_json_with_duration(frame_number: usize, duration_secs: i32) -> String {
    let mut value: serde_json::Value =
        serde_json::from_str(&frame_prompt_json(frame_number)).unwrap();
    value["frames"][0]["duration_secs"] = json!(duration_secs);
    value.to_string()
}

fn boundary_frame_prompt_replies() -> Vec<String> {
    vec![
        frame_prompt_json(1),
        frame_prompt_json(2),
        frame_prompt_json(3),
        frame_prompt_json(4),
    ]
}

fn boundary_frame_generation_replies() -> Vec<String> {
    let mut replies = vec![
        BEAT_EXTRACTION_JSON.to_string(),
        SHOT_ANALYSIS_JSON.to_string(),
        segment_plan_json_for_shot(1, None),
        segment_plan_json_for_shot(2, None),
    ];
    replies.extend(boundary_frame_prompt_replies());
    replies
}

fn single_segment_frame_prompt_replies(duration_secs: i32) -> Vec<String> {
    (1..=2)
        .map(|frame_number| frame_prompt_json_with_duration(frame_number, duration_secs))
        .collect()
}

async fn generate_chat_video_frame_prompt_stage(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    engine: &TeraPromptEngine,
) {
    let mut replies = vec![
        BEAT_EXTRACTION_JSON.to_string(),
        SHOT_ANALYSIS_JSON.to_string(),
        segment_plan_json_for_shot(1, None),
        segment_plan_json_for_shot(2, None),
    ];
    replies.extend(boundary_frame_prompt_replies());
    generate_chat_video_frame_prompt_stage_with_replies(
        service, project_id, chapter_id, engine, replies,
    )
    .await;
}

async fn generate_chat_video_frame_prompt_stage_from_existing_plan(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    engine: &TeraPromptEngine,
) {
    generate_chat_video_frame_prompt_stage_with_replies(
        service,
        project_id,
        chapter_id,
        engine,
        boundary_frame_prompt_replies(),
    )
    .await;
}

async fn generate_chat_video_frame_prompt_stage_with_replies(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
    engine: &TeraPromptEngine,
    replies: Vec<String>,
) {
    service
        .generate_chapter_chat_video_frame_prompts(
            prompt_stage_input(
                project_id,
                chapter_id,
                "chat_video",
                &SequenceLlmProvider::new_owned(replies),
                engine,
                frame_generation_options(),
                None,
            ),
            None,
            None,
        )
        .await
        .unwrap();
}

async fn insert_boundary_frame_images(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> HashMap<i32, i64> {
    let frame_rows = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_prompt"))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db)
        .await
        .unwrap();
    let mut image_id_by_frame_number = HashMap::new();
    let source_metadata = video_source_metadata(&generation_source());
    for row in frame_rows {
        let image = insert_generated_image_with_binding(
            db,
            project_id,
            Some(chapter_id),
            Some(json!({
                "generation_context": {
                    "source": "video_boundary_frame_prompt",
                    "prompt_id": row.id
                },
                "source": source_metadata.clone()
            })),
        )
        .await;
        image_id_by_frame_number.insert(row.shot_number, image.id);
    }
    image_id_by_frame_number
}

fn generation_source() -> ChapterVideoSourceContext<'static> {
    ChapterVideoSourceContext {
        content: "小宦官推门入殿，忽然察觉暗处有人，回头压住恐惧。",
        summary: Some("小宦官夜入宫殿并察觉危险。"),
        analysis: None,
    }
}

fn empty_prompt_variables() -> &'static HashMap<String, Value> {
    static EMPTY: OnceLock<HashMap<String, Value>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

fn prompt_stage_input<'a>(
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &'static str,
    llm: &'a dyn LlmProvider,
    prompt_engine: &'a dyn a4n_engine::prompt::engine::PromptEngine,
    options: ChapterVideoPromptOptions,
    cancel_token: Option<&'a CancellationToken>,
) -> ChapterVideoPromptStageInput<'a, 'static> {
    ChapterVideoPromptStageInput {
        project_id,
        chapter_id,
        workflow_mode,
        source: generation_source(),
        llm,
        prompt_engine,
        variables: empty_prompt_variables(),
        options,
        cancel_token,
    }
}

fn first_frame_chain_source_metadata() -> Value {
    video_source_metadata(&generation_source())
}

fn empty_video_generation_asset_fingerprint() -> Value {
    let available_assets = json!([]);
    json!({
        "schema_version": 1,
        "reference_image_ids": [],
        "video_library_image_ids": [],
        "available_assets_sha256": source_content_sha256(&available_assets.to_string())
    })
}

fn stored_first_frame_chain_segment_plan() -> Value {
    json!({
        "segments": [
            {
                "segment_number": 1,
                "analysis_shot_id": 1,
                "segment_index_in_shot": 1,
                "shot_segment_count": 1,
                "covered_beat_ids": [1, 2],
                "title": "入殿",
                "scene_summary": "小宦官推门入殿，发现异常",
                "blocking_summary": "小宦官从殿门外进入殿内，门是固定空间锚点，人物始终在画面中央偏左",
                "spatial_anchors": [{"anchor": "殿门", "usage": "约束入殿起点和移动方向"}],
                "emotion_action_goals": [{"character": "小宦官", "emotion": "紧张", "visible_action": "放慢脚步并收紧肩膀"}],
                "video_intent": {"hook": "推门入殿后察觉异常", "audience_effect": "让观众感到紧张和不安", "rhythm": "缓慢进入后短暂停顿", "visual_priority": "小宦官推门、入殿、发现异常的动作"},
                "emotional_arc": {"start": "谨慎", "turn": "察觉殿内异常", "end": "紧张压住恐惧", "function": "场景建立"},
                "material_coverage": {"required_entities": ["小宦官", "殿门", "殿内空间"], "missing_or_weak_references": ["小宦官", "殿门", "殿内空间"], "risk_note": "需要用文字明确人物服饰、殿门和殿内空间"},
                "source_span_ids": ["ch1:s1:f1", "ch1:s1:f2"],
                "duration_secs": 4,
                "aspect_ratio": "16:9",
                "reference_image_ids": []
            },
            {
                "segment_number": 2,
                "analysis_shot_id": 2,
                "segment_index_in_shot": 1,
                "shot_segment_count": 1,
                "covered_beat_ids": [3],
                "title": "回望",
                "scene_summary": "小宦官回望暗处，压住恐惧",
                "blocking_summary": "小宦官从殿内中景转向暗廊，暗廊位于画面右后方，人物不要跳到暗廊另一侧",
                "spatial_anchors": [{"anchor": "暗廊入口", "usage": "约束回望方向和终点视线"}],
                "emotion_action_goals": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "停步、回头、轻微后缩"}],
                "video_intent": {"hook": "小宦官回望暗处压住恐惧", "audience_effect": "制造悬疑和压迫", "rhythm": "停顿后缓慢回望", "visual_priority": "回头、后缩和暗廊方向"},
                "emotional_arc": {"start": "紧张", "turn": "听见或感到暗处异动", "end": "恐惧但克制", "function": "悬念保留"},
                "material_coverage": {"required_entities": ["小宦官", "暗廊入口"], "missing_or_weak_references": ["小宦官", "暗廊入口"], "risk_note": "需要用文字锁定暗廊在画面右后方"},
                "source_span_ids": ["ch1:s1:f3"],
                "duration_secs": 4,
                "aspect_ratio": "16:9",
                "reference_image_ids": []
            }
        ]
    })
}

fn stored_initial_first_frame_prompt() -> Value {
    json!({
        "frame_number": 1,
        "title": "殿门外首帧",
        "scene_summary": "小宦官站在殿门外准备推门",
        "frame_prompt": "【参考图约束】无参考图。【画面主体】小宦官站在紧闭宫门前的走廊中央偏左，右手抬起准备推门。【空间与道具】朱红殿门、铜钉、门槛和两侧廊柱固定在画面内。【构图光线风格】平视中景，右侧烛光斜照，电影感写实。【一致性与避免项】保持人物、殿门、廊柱和光源方向连续。",
        "blocking_state": {
            "characters": [{"name": "小宦官", "position": "殿门外中央偏左", "facing": "面向殿门", "pose": "抬手准备推门"}],
            "spatial_anchors": ["殿门", "门槛", "廊柱"],
            "do_not_change": ["殿门始终在人物前方"]
        },
        "lighting_state": {"time_of_day": "夜晚", "key_light": "烛光", "direction": "画面右侧", "shadow": "柔和昏暗阴影", "intent": "紧张"},
        "negative_prompt": "低清晰度、画面模糊、人体结构扭曲、多余肢体、面部不一致、空间关系错误",
        "duration_secs": 4,
        "aspect_ratio": "16:9",
        "reference_image_ids": [],
        "segment_numbers": [1]
    })
}

fn saved_first_frame_chain_video_prompt_metadata(
    source_metadata: &Value,
    generation_asset_fingerprint: &Value,
    prompt_output_language: &str,
) -> Value {
    json!({
        "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
        "shot": {"shot_number": 1, "title": "入殿", "scene_summary": "小宦官推门入殿，发现异常"},
        "video_unit": {
            "unit_number": 1,
            "unit_type": "content",
            "title": "入殿",
            "duration_secs": 4,
            "source_segment_number": 1,
            "from_segment_number": null,
            "to_segment_number": null,
            "start_frame_number": null,
            "end_frame_number": null,
            "storyboard_number": 1,
            "source_segments": [stored_first_frame_chain_segment_plan()["segments"][0].clone()]
        },
        "segment": stored_first_frame_chain_segment_plan()["segments"][0].clone(),
        "start_first_frame": stored_initial_first_frame_prompt(),
        "available_assets": [],
        "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
        "self_contained_context": {
            "must_repeat_from_previous": [],
            "must_establish_in_segment": ["小宦官推门入殿，发现异常"],
            "must_preserve_throughout": ["人物身份、服装、殿门、暗角方向和光源方向必须连续"],
            "handoff_to_next": "小宦官停在殿内中央偏左并看向右后方暗角",
            "omission_risks": ["省略殿门和暗角会导致空间跳变"]
        },
        "start_state": {"character": "小宦官", "position": "殿门外中央偏左", "pose": "准备推门", "camera": "平视中景", "lighting": "右侧烛光"},
        "end_state": {"character": "小宦官", "position": "殿内中央偏左", "pose": "看向右后方暗角", "camera": "平视中景稳定", "lighting": "右侧烛光连续"},
        "transition_in": {"from_previous_end": "第一片段从章节当前起点建立画面，不编造上一段尾态", "to_current_start": "殿门外中央偏左", "cut_type": "连续承接", "camera_transition": "固定机位轻推", "first_beat_action": "抬手触门"},
        "blocking_plan": {"start": "殿门外中央偏左", "movement": "推门进入殿内", "end": "殿内中央偏左并看向右后方暗角", "spatial_anchors": ["殿门", "暗角"], "do_not_change": ["左右关系和门的位置"]},
        "lighting_plan": {"key_light": "烛光", "direction": "画面右侧", "shadow": "柔和昏暗阴影", "continuity": "全片段保持同一光源逻辑"},
        "emotion_action_beats": [{"character": "小宦官", "emotion": "紧张", "visible_action": "推门、入殿、停步观察"}],
        "dialogue_beats": [],
        "cinematography_plan": {"shot_size": "中景", "camera_angle": "平视", "camera_movement": "缓慢推进", "visual_priority": "推门入殿和察觉异常"},
        "temporal_beats": [{
            "start_sec": 0,
            "end_sec": 4,
            "subject_action": "推门进入并停步回望",
            "camera": "平视中景缓慢推进",
            "continuity_target": "殿内中央偏左看向右后方暗角"
        }],
        "reference_binding": {"bindings": [], "coverage_check": "参考一致性由文字约束承担"},
        "continuity_check": {"must_keep": ["小宦官身份", "殿门", "右后方暗角"], "handoff": "殿内中央偏左看向右后方暗角"},
        "source_prompt_type": "frame_segment_plan",
        "first_frame_chain": {
            "start_source": "generated_first_frame",
            "first_frame_prompt_number": 1,
            "previous_video_shot_number": null
        },
        "source": source_metadata.clone(),
        "prompt_output_language": prompt_output_language
    })
}

async fn store_first_frame_chain_prompt_seed(
    service: &VideoService,
    project_id: i64,
    chapter_id: i64,
) -> chapter_video_prompt::Model {
    let source_metadata = first_frame_chain_source_metadata();
    let generation_asset_fingerprint = empty_video_generation_asset_fingerprint();
    let segment_plan = stored_first_frame_chain_segment_plan();
    let segment_plan_raw = segment_plan.to_string();
    let first_frame = stored_initial_first_frame_prompt();

    service
        .sync_chapter_prompts(
            chapter_id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::BoundaryFrames,
                style: Some("电影感写实"),
                prompt_type: "frame_segment_plan",
                shot_number: 0,
                title: Some("视频片段规划"),
                scene_summary: None,
                visual_prompt: &segment_plan_raw,
                motion_prompt: None,
                negative_prompt: None,
                duration_secs: 4,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: Some(json!([])),
                metadata_json: Some(json!({
                    "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
                    "raw": segment_plan_raw,
                    "segments": segment_plan["segments"].clone(),
                    "available_assets": [],
                    "source": source_metadata.clone()
                })),
            })],
        )
        .await
        .unwrap();

    service
        .sync_chapter_prompts(
            chapter_id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::FirstFrameChain,
                style: Some("电影感写实"),
                prompt_type: "first_frame_prompt",
                shot_number: 1,
                title: Some(first_frame["title"].as_str().unwrap()),
                scene_summary: Some(first_frame["scene_summary"].as_str().unwrap()),
                visual_prompt: first_frame["frame_prompt"].as_str().unwrap(),
                motion_prompt: None,
                negative_prompt: Some(first_frame["negative_prompt"].as_str().unwrap()),
                duration_secs: 4,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: Some(json!([])),
                metadata_json: Some(json!({
                    "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
                    "first_frame": first_frame,
                    "blocking_state": stored_initial_first_frame_prompt()["blocking_state"].clone(),
                    "lighting_state": stored_initial_first_frame_prompt()["lighting_state"].clone(),
                    "available_assets": [],
                    "generation_asset_fingerprint": generation_asset_fingerprint.clone(),
                    "source_prompt_type": "frame_segment_plan",
                    "source": source_metadata.clone(),
                    "prompt_output_language": "zh_cn"
                })),
            })],
        )
        .await
        .unwrap();

    let seed_video_prompt: serde_json::Value = serde_json::from_str(&video_shot_json(1)).unwrap();
    let seed_video_prompt = seed_video_prompt["shots"][0].as_object().unwrap();
    let seed_visual_prompt = seed_video_prompt["visual_prompt"].as_str().unwrap();
    let seed_motion_prompt = seed_video_prompt["motion_prompt"].as_str().unwrap();
    let seed_negative_prompt = seed_video_prompt["negative_prompt"].as_str().unwrap();

    let video_rows = service
        .sync_chapter_prompts(
            chapter_id,
            vec![new_prompt_draft(NewChapterVideoPromptInput {
                project_id,
                chapter_id,
                workflow_mode: PromptWorkflowMode::FirstFrameChain,
                style: Some("电影感写实"),
                prompt_type: "video_prompt",
                shot_number: 1,
                title: Some("入殿"),
                scene_summary: Some("小宦官推门入殿，发现异常"),
                visual_prompt: seed_visual_prompt,
                motion_prompt: Some(seed_motion_prompt),
                negative_prompt: Some(seed_negative_prompt),
                duration_secs: 4,
                aspect_ratio: Some("16:9"),
                reference_image_ids_json: Some(json!([])),
                metadata_json: Some(saved_first_frame_chain_video_prompt_metadata(
                    &source_metadata,
                    &generation_asset_fingerprint,
                    "zh_cn",
                )),
            })],
        )
        .await
        .unwrap();

    video_rows
        .into_iter()
        .find(|row| row.prompt_type == "video_prompt" && row.shot_number == 1)
        .expect("seeded zh video prompt should be saved")
}

mod boundary_video;
mod chat_video;
mod first_frame_chain;
mod frame_prompts;
mod video_prompts;
