import { apiInvoke } from "../index";
import type {
  ImportProjectTextUploadInput,
  ImportReviewSummary,
} from "./types";

export async function importProjectTextUpload(
  input: ImportProjectTextUploadInput,
): Promise<ImportReviewSummary[]> {
  return apiInvoke<ImportReviewSummary[]>("import_project_text_upload", { input });
}
