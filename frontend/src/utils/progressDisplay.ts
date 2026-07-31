type ProgressDetailSource = "stream" | "message" | "none";

export interface ProgressDisplayText {
  stage: string;
  detail: string | null;
  detailSource: ProgressDetailSource;
}

interface ProgressDisplayInput {
  message?: string | null;
  streamText?: string | null;
  stageHint?: string | null;
  fallbackStage?: string;
}

const DEFAULT_STAGE = "任务执行中";
const MAX_STREAM_LINE_CHARS = 180;

const normalizeProgressText = (text?: string | null): string =>
  (text ?? "").replace(/\s+/g, " ").trim();

export const streamLineFromBuffer = (buffer: string): string => {
  const compact = normalizeProgressText(buffer);
  const chars = Array.from(compact);
  return chars.length > MAX_STREAM_LINE_CHARS
    ? `…${chars.slice(-MAX_STREAM_LINE_CHARS).join("")}`
    : compact;
};

const sameProgressText = (left?: string | null, right?: string | null): boolean =>
  normalizeProgressText(left) === normalizeProgressText(right);

const splitStageAndDetail = (
  message: string,
): { stage: string; detail: string } | null => {
  const separatorIndex = firstStageSeparatorIndex(message);
  if (separatorIndex <= 0) return null;

  const stage = message.slice(0, separatorIndex).trim();
  const detail = message.slice(separatorIndex + 1).trim();
  if (!stage || !detail || sameProgressText(stage, detail)) return null;
  return { stage, detail };
};

const firstStageSeparatorIndex = (message: string): number => {
  const candidates = ["：", ":"]
    .map((separator) => message.indexOf(separator))
    .filter((index) => index > 0);
  if (candidates.length === 0) return -1;

  const index = Math.min(...candidates);
  const prefix = message.slice(0, index).trim();
  const suffix = message.slice(index + 1).trim();
  if (!prefix || !suffix) return -1;

  // 避免把 URL、时间戳或很长错误片段误拆成阶段。
  if (/https?$/.test(prefix) || /^\d{1,2}$/.test(prefix) || prefix.length > 32) {
    return -1;
  }
  return index;
};

const inferProgressStage = (message: string): string | null => {
  if (/模型已开始输出|开始输出|流式输出/.test(message)) return "模型输出中";
  if (/准备|构建|加载|收集|整理上下文|读取/.test(message)) return "准备生成上下文";
  if (/调用.*(LLM|模型|API)|请求.*模型|等待.*模型|模型调用/.test(message)) {
    return "调用生成模型";
  }
  if (/解析|提取|结构化|校验|验证|序列化/.test(message)) return "解析与校验结果";
  if (/保存|写入|同步|入库|更新|落库/.test(message)) return "保存生成结果";
  if (/图片|图像|立绘|首帧|尾帧|边界帧/.test(message)) return "生成图片资产";
  if (/视频|镜头|片段规划|视频提示词|运镜/.test(message)) return "生成视频资产";
  if (/漫画|分镜|画格|面板/.test(message)) return "生成漫画资产";
  if (/导入|拆分|去重|审核/.test(message)) return "导入项目分析";
  if (/章节|正文|大纲|蓝图|档案|提示词|分析/.test(message)) return "生成文字资产";
  if (/停止|取消/.test(message)) return "停止任务";
  return null;
};

export const progressDisplayText = ({
  message,
  streamText,
  stageHint,
  fallbackStage = DEFAULT_STAGE,
}: ProgressDisplayInput): ProgressDisplayText => {
  const normalizedMessage = normalizeProgressText(message);
  const normalizedStreamText = normalizeProgressText(streamText);
  const normalizedStageHint = normalizeProgressText(stageHint);
  const splitMessage = normalizedMessage ? splitStageAndDetail(normalizedMessage) : null;

  const stage =
    normalizedStageHint ||
    splitMessage?.stage ||
    (normalizedMessage ? inferProgressStage(normalizedMessage) : null) ||
    normalizeProgressText(fallbackStage) ||
    DEFAULT_STAGE;

  if (
    normalizedStreamText &&
    !sameProgressText(normalizedStreamText, normalizedMessage) &&
    !sameProgressText(normalizedStreamText, stage)
  ) {
    return {
      stage,
      detail: normalizedStreamText,
      detailSource: "stream",
    };
  }

  const messageDetail =
    splitMessage?.detail ||
    (normalizedMessage && !sameProgressText(normalizedMessage, stage)
      ? normalizedMessage
      : null);

  return {
    stage,
    detail: messageDetail,
    detailSource: messageDetail ? "message" : "none",
  };
};
