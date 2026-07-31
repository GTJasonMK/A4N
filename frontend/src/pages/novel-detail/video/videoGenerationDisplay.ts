import type { GeneratedImage } from "../../../api/image";
import type { ChapterVideoPrompt } from "../../../api/video";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { videoImageGenerationSource } from "./videoTabUtils";

export const promptTitle = (prompt: ChapterVideoPrompt): string => {
  const title = prompt.title?.trim();
  return title ? title : `镜头 ${prompt.shot_number}`;
};

export const isVideoTailFrameImage = (image: GeneratedImage): boolean =>
  videoImageGenerationSource(image) === "video_tail_frame_capture" ||
  image.image_type === "video_tail_frame";

const labelForKey = (key: string): string => {
  const labels: Record<string, string> = {
    start: "起点",
    movement: "走位",
    end: "终点",
    spatial_anchors: "固定参照物",
    video_intent: "视频意图",
    hook: "钩子",
    audience_effect: "观众感受",
    rhythm: "节奏",
    visual_priority: "视觉重点",
    emotional_arc: "情绪弧线",
    turn: "转折",
    function: "段落功能",
    material_coverage: "素材覆盖",
    required_entities: "关键对象",
    missing_or_weak_references: "缺失/弱覆盖",
    risk_note: "风险说明",
    do_not_change: "不得改变",
    key_light: "主光",
    direction: "方向",
    shadow: "阴影",
    continuity: "连续性",
    character: "人物",
    emotion: "情绪",
    visible_action: "可见动作",
    timing: "时机",
    shot_size: "景别",
    camera_angle: "机位",
    camera_movement: "镜头运动",
    axis_rule: "轴线规则",
    lens_hint: "焦段",
    start_sec: "开始秒",
    end_sec: "结束秒",
    camera: "镜头",
    subject_action: "主体动作",
    continuity_target: "连续性目标",
    image_id: "参考图",
    entity: "对象",
    role: "角色",
    usage: "用途",
    must_keep: "必须保持",
    coverage_check: "覆盖检查",
    blocking: "站位",
    lighting: "光影",
    references: "参考",
    revision: "修订",
    speaker: "说话人",
    line: "台词",
    delivery: "语气",
    listener_reaction: "听者反应",
    sound_design: "声音设计",
    ambience: "环境声",
    action_sounds: "动作声",
    dialogue_delivery: "对白语气",
    silence_or_pause: "静默/停顿",
  };
  return labels[key] ?? key;
};

const metadataValueText = (value: unknown): string => {
  if (value == null) return "";
  if (typeof value === "string") return value.trim();
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) {
    return value.map(metadataValueText).filter(Boolean).join("、");
  }
  const record = asRecord(value);
  if (!record) return "";
  return Object.entries(record)
    .map(([key, item]) => {
      const text = metadataValueText(item);
      return text ? `${labelForKey(key)}：${text}` : "";
    })
    .filter(Boolean)
    .join("；");
};

const metadataFieldText = (record: Record<string, unknown>, key: string): string => {
  return metadataValueText(record[key]);
};

type ClauseSpec = [key: string, label: string];

const clausesText = (record: Record<string, unknown>, clauses: ClauseSpec[]): string => {
  return clauses
    .map(([key, label]) => {
      const text = metadataFieldText(record, key);
      return text ? `${label}：${text}` : "";
    })
    .filter(Boolean)
    .join("；");
};

const clauseListText = (
  value: unknown,
  clauses: ClauseSpec[],
  prefix: (record: Record<string, unknown>, index: number) => string,
): string => {
  if (!Array.isArray(value)) return "";
  return value
    .map((item, index) => {
      const itemRecord = asRecord(item);
      if (!itemRecord) return metadataValueText(item);
      const text = clausesText(itemRecord, clauses);
      return text ? `${prefix(itemRecord, index)}${text}` : "";
    })
    .filter(Boolean)
    .join("\n");
};

const sectionText = (key: string, value: unknown): string => {
  const record = asRecord(value);
  if (key === "blocking_plan" && record) {
    return clausesText(record, [
      ["start", "起点"],
      ["movement", "走位"],
      ["end", "终点"],
      ["spatial_anchors", "固定参照物"],
      ["do_not_change", "不得改变"],
    ]);
  }
  if (key === "lighting_plan" && record) {
    return clausesText(record, [
      ["key_light", "主光"],
      ["direction", "方向"],
      ["shadow", "阴影"],
      ["continuity", "连续性"],
    ]);
  }
  if (key === "cinematography_plan" && record) {
    return clausesText(record, [
      ["shot_size", "景别"],
      ["camera_angle", "机位"],
      ["camera_movement", "镜头运动"],
      ["axis_rule", "轴线规则"],
      ["lens_hint", "焦段"],
    ]);
  }
  if (key === "continuity_check" && record) {
    return clausesText(record, [
      ["blocking", "站位"],
      ["lighting", "光影"],
      ["emotion", "情绪"],
      ["references", "参考"],
      ["revision", "修订"],
    ]);
  }
  if (key === "emotion_action_beats" && Array.isArray(value)) {
    return clauseListText(
      value,
      [
        ["character", "人物"],
        ["emotion", "情绪"],
        ["visible_action", "可见动作"],
        ["timing", "时机"],
      ],
      (_itemRecord, index) => `${index + 1}. `,
    );
  }
  if (key === "dialogue_beats" && Array.isArray(value)) {
    return clauseListText(
      value,
      [
        ["speaker", "说话人"],
        ["line", "台词"],
        ["delivery", "语气"],
        ["visible_action", "可见动作"],
        ["listener_reaction", "听者反应"],
        ["timing", "时机"],
      ],
      (_itemRecord, index) => `${index + 1}. `,
    );
  }
  if (key === "temporal_beats" && Array.isArray(value)) {
    return clauseListText(
      value,
      [
        ["camera", "镜头"],
        ["subject_action", "主体动作"],
        ["continuity_target", "连续性目标"],
      ],
      (itemRecord, index) => {
        const start = metadataFieldText(itemRecord, "start_sec");
        const end = metadataFieldText(itemRecord, "end_sec");
        return start && end ? `${start}-${end} 秒：` : `时间段 ${index + 1}：`;
      },
    );
  }
  if (key === "reference_binding" && record) {
    const bindings = Array.isArray(record.bindings) ? record.bindings : [];
    const lines = bindings
      .map((item) => {
        const binding = asRecord(item);
        if (!binding) return metadataValueText(item);
        const id = metadataFieldText(binding, "image_id");
        const text = clausesText(binding, [
          ["entity", "对象"],
          ["usage", "用途"],
          ["must_keep", "必须保持"],
          ["role", "角色"],
        ]);
        return `${id ? `参考图 #${id}` : "参考图"}${text ? `：${text}` : ""}`;
      })
      .filter(Boolean);
    const coverage = metadataFieldText(record, "coverage_check");
    if (coverage) lines.push(`覆盖检查：${coverage}`);
    return lines.join("\n");
  }
  return metadataValueText(value);
};

const directorSectionValue = (metadata: unknown, key: string): unknown => {
  const root = asRecord(metadata);
  const shot = asRecord(root?.shot);
  const segment = asRecord(root?.segment);
  const chatVideo = asRecord(root?.chat_video);
  return root?.[key] ?? shot?.[key] ?? segment?.[key] ?? chatVideo?.[key] ?? null;
};

export const directorSections = (
  metadata: unknown,
): Array<{ key: string; label: string; text: string }> => {
  const sections = [
    ["video_intent", "视频意图"],
    ["emotional_arc", "情绪弧线"],
    ["material_coverage", "素材覆盖"],
    ["self_contained_context", "独立上下文"],
    ["start_state", "起始状态"],
    ["transition_in", "入场转场"],
    ["end_state", "结尾状态"],
    ["blocking_plan", "站位/走位"],
    ["lighting_plan", "光影"],
    ["emotion_action_beats", "情绪动作"],
    ["dialogue_beats", "对白"],
    ["cinematography_plan", "镜头语法"],
    ["temporal_beats", "时间节拍"],
    ["reference_binding", "素材绑定"],
    ["sound_design", "声音设计"],
    ["continuity_check", "自检"],
  ] as const;
  return sections
    .map(([key, label]) => ({
      key,
      label,
      text: sectionText(key, directorSectionValue(metadata, key)),
    }))
    .filter((section) => section.text.length > 0);
};
