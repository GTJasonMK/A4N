import { apiInvoke } from "../index";
import type { ImportProjectTextUploadInput, ImportReview } from "./types";

export async function importProjectTextUpload(
  input: ImportProjectTextUploadInput,
): Promise<ImportReview[]> {
  return apiInvoke<ImportReview[]>("import_project_text_upload", { input });
}
