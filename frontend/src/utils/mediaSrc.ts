import { toRuntimeMediaSrc } from "../api/runtime";

export function toMediaSrc(filePath?: string | null): string {
  return toRuntimeMediaSrc(filePath);
}
