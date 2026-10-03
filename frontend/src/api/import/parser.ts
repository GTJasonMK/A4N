import { apiInvoke } from "../index";
import type {
  CanvasTextAssetSummary,
  ImportProjectTextUploadInput,
  ImportReviewSummary,
} from "../generated/host";

export async function importProjectTextUpload(
  input: ImportProjectTextUploadInput,
): Promise<ImportReviewSummary[]> {
  return apiInvoke<ImportReviewSummary[]>("import_project_text_upload", { input });
}

export async function importProjectTextAssetUpload(
  input: ImportProjectTextUploadInput,
): Promise<CanvasTextAssetSummary> {
  return apiInvoke<CanvasTextAssetSummary>("import_project_text_asset_upload", { input });
}
