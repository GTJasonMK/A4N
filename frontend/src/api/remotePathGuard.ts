import { shouldUseHttpBackend } from "../utils/runtimeMode";

const URL_LIKE_RE = /^(?:https?:|data:|blob:|a4n-media:)/i;
const ABSOLUTE_LOCAL_PATH_RE = /^(?:\/|~\/|[a-zA-Z]:[\\/]|\\\\)/;

function assertRemoteBackendCanUsePath(path: string, label: string): void {
  const value = path.trim();
  if (!value || !shouldUseHttpBackend()) return;
  if (URL_LIKE_RE.test(value)) return;
  if (ABSOLUTE_LOCAL_PATH_RE.test(value)) {
    throw new Error(
      `${label} 是本机路径，远程服务器无法直接读取。请先上传为云端资产，或使用服务器可访问的 URL。`,
    );
  }
}

export function assertRemoteBackendCanUsePaths(paths: string[], label: string): void {
  paths.forEach((path, index) => {
    assertRemoteBackendCanUsePath(path, `${label} #${index + 1}`);
  });
}
