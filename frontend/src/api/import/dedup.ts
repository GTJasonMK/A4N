import { apiInvoke } from "../index";
import type { CheckImportDedupInput, DedupResult } from "./types";

export async function checkImportDedup(input: CheckImportDedupInput): Promise<DedupResult> {
  return apiInvoke<DedupResult>("check_import_dedup", {
    input,
  });
}
