import type { LogReadResult } from "../../../api/logs";

export type { LogReadResult };

export const logPathText = (
  filePath: string | null | undefined,
  logDir: string,
): string => {
  const explicitPath = filePath?.trim();
  if (explicitPath) return explicitPath;
  const configuredDir = logDir.trim();
  return configuredDir ? configuredDir : "日志路径未返回";
};

export const logLineClassName = (line: string): string => {
  if (line.includes(" ERROR ") || line.includes("[error]")) return "text-error";
  if (line.includes(" WARN ") || line.includes("[warn]")) return "text-warning";
  if (line.includes(" DEBUG ") || line.includes("[debug]"))
    return "text-book-text-muted";
  if (line.includes(" TRACE ") || line.includes("[trace]")) {
    return "text-book-text-muted opacity-70";
  }
  return "text-book-text-main";
};
