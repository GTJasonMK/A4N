import { apiInvoke } from "./index";
import type {
  BatchExportInput,
  ExportFileData,
  ExportProjectInput,
  ExportPublicationInput,
} from "./generated/host";

export type {
  BatchExportFormat,
  BatchExportInput,
  ExportFileData,
  ExportProjectInput,
  ExportPublicationInput,
  ExportPublicationMetadata,
} from "./generated/host";

const exportArgs = (input: unknown, taskId: string) => ({
  input,
  taskId,
});

export function exportTxtData(
  input: ExportProjectInput,
  taskId: string,
): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>(
    "export_txt_data",
    exportArgs(input, taskId),
  );
}

export function exportOutlineJsonData(
  input: ExportProjectInput,
  taskId: string,
): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>(
    "export_outline_json_data",
    exportArgs(input, taskId),
  );
}

export function exportCharactersJsonData(
  input: ExportProjectInput,
  taskId: string,
): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>(
    "export_characters_json_data",
    exportArgs(input, taskId),
  );
}

export function exportMarkdownData(
  input: ExportPublicationInput,
  taskId: string,
): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>(
    "export_markdown_data",
    exportArgs(input, taskId),
  );
}

export function exportEpubData(
  input: ExportPublicationInput,
  taskId: string,
): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>(
    "export_epub_data",
    exportArgs(input, taskId),
  );
}

export function exportPdfData(
  input: ExportProjectInput,
  taskId: string,
): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>(
    "export_pdf_data",
    exportArgs(input, taskId),
  );
}

export function exportBatchData(
  input: BatchExportInput,
  taskId: string,
): Promise<ExportFileData[]> {
  return apiInvoke<ExportFileData[]>(
    "export_batch_data",
    exportArgs(input, taskId),
  );
}
