import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { CheckImportDedupInput, DedupResult } from "./types";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法检查导入内容");
  return id;
};

export async function checkImportDedup(input: CheckImportDedupInput): Promise<DedupResult> {
  return apiInvoke<DedupResult>("check_import_dedup", {
    userId: currentUserId(),
    input,
  });
}
