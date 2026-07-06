import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import { idNum, type ApiId as Id } from "./shared";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法导出项目");
  return id;
};

export interface ExportFileData {
  file_name: string;
  mime_type: string;
  data_base64: string;
}

export async function exportTxtData(projectId: Id, taskId?: string | null): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>("export_txt_data", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    taskId: taskId ?? null,
  });
}

export async function exportOutlineJsonData(
  projectId: Id,
  taskId?: string | null,
): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>("export_outline_json_data", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    taskId: taskId ?? null,
  });
}

export async function exportCharactersJsonData(
  projectId: Id,
  taskId?: string | null,
): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>("export_characters_json_data", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    taskId: taskId ?? null,
  });
}

export async function exportMarkdownData(
  projectId: Id,
  input: { title: string; author: string },
  taskId?: string | null,
): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>("export_markdown_data", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input,
    taskId: taskId ?? null,
  });
}

export async function exportEpubData(
  projectId: Id,
  title: string,
  author: string,
  taskId?: string | null,
): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>("export_epub_data", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    title,
    author,
    taskId: taskId ?? null,
  });
}

export async function exportPdfData(projectId: Id, taskId?: string | null): Promise<ExportFileData> {
  return apiInvoke<ExportFileData>("export_pdf_data", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    taskId: taskId ?? null,
  });
}

export async function exportBatchData(input: {
  project_id: number;
  formats: string[];
  title: string;
  author: string;
}, taskId?: string | null): Promise<ExportFileData[]> {
  return apiInvoke<ExportFileData[]>("export_batch_data", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
  });
}
