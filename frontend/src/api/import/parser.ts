import { apiInvoke } from "../index";
import type { ImportFileResult, ImportUploadedFileInput } from "./types";

export async function importUploadedFile(
  input: ImportUploadedFileInput,
): Promise<ImportFileResult> {
  return apiInvoke<ImportFileResult>("import_uploaded_file", { input });
}
