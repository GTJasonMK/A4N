const formatNumber = (value: number, maximumFractionDigits: number): string =>
  value.toLocaleString("zh-CN", {
    minimumFractionDigits: 0,
    maximumFractionDigits,
  });

export function formatSecondsLabel(
  seconds: number,
  maximumFractionDigits = 0,
): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "时长未知";
  return `${formatNumber(seconds, maximumFractionDigits)} 秒`;
}

export function formatMediaDuration(
  seconds: number,
  maximumFractionDigits = 0,
): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0 秒";
  const precision = 10 ** maximumFractionDigits;
  const normalizedSeconds = Math.round(seconds * precision) / precision;
  if (normalizedSeconds < 60) {
    return formatSecondsLabel(normalizedSeconds, maximumFractionDigits);
  }

  const minutes = Math.floor(normalizedSeconds / 60);
  const remainingSeconds = normalizedSeconds - minutes * 60;
  if (remainingSeconds <= 0) return `${minutes} 分`;

  const remainingText = formatNumber(remainingSeconds, maximumFractionDigits);
  return `${minutes} 分 ${remainingSeconds < 10 ? "0" : ""}${remainingText} 秒`;
}

export function formatDurationMilliseconds(milliseconds: number | null): string {
  if (milliseconds == null || !Number.isFinite(milliseconds) || milliseconds < 0) {
    return "—";
  }
  if (milliseconds < 1000) {
    return `${formatNumber(milliseconds, 0)} 毫秒`;
  }
  return formatMediaDuration(milliseconds / 1000, 1);
}

export function formatFrameRate(fps: number): string {
  if (!Number.isFinite(fps) || fps <= 0) return "帧率未知";
  return `${formatNumber(fps, 2)} 帧/秒`;
}

export function formatMediaDimensions(value: string): string {
  const match = value.trim().match(/^(\d+)\s*[x×]\s*(\d+)$/i);
  return match ? `${match[1]}×${match[2]}` : value.trim() || "尺寸未设置";
}

export function formatVideoDimensions(value: string): string {
  return formatMediaDimensions(value);
}
