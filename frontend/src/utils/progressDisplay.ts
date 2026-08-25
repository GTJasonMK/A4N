type ProgressDetailSource = "stream" | "message" | "none";

interface ProgressDisplayText {
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

export const progressDisplayText = ({
  message,
  streamText,
  stageHint,
  fallbackStage = "",
}: ProgressDisplayInput): ProgressDisplayText => {
  const normalizedMessage = normalizeProgressText(message);
  const normalizedStreamText = normalizeProgressText(streamText);
  const normalizedStageHint = normalizeProgressText(stageHint);
  const stage = normalizedStageHint || normalizeProgressText(fallbackStage);

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
    normalizedMessage && !sameProgressText(normalizedMessage, stage) ? normalizedMessage : null;

  return {
    stage,
    detail: messageDetail,
    detailSource: messageDetail ? "message" : "none",
  };
};
