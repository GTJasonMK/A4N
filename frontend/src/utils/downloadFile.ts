import { base64ToBytes } from "./base64";

interface DownloadableBase64File {
  file_name: string;
  mime_type: string;
  data_base64: string;
}

export function base64DataToBlob(
  dataBase64: string,
  mimeType: string,
): Blob {
  const bytes = base64ToBytes(dataBase64);
  return new Blob([bytes], { type: mimeType });
}

export function downloadBlob(fileName: string, blob: Blob): void {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  try {
    anchor.href = url;
    anchor.download = fileName;
    anchor.rel = "noopener";
    document.body.appendChild(anchor);
    anchor.click();
  } finally {
    anchor.remove();
    URL.revokeObjectURL(url);
  }
}

export function downloadTextFile(
  fileName: string,
  content: string,
  mimeType = "text/plain;charset=utf-8",
): void {
  downloadBlob(fileName, new Blob([content], { type: mimeType }));
}

export function downloadBase64ExportFile(file: DownloadableBase64File): void {
  downloadBlob(
    file.file_name,
    base64DataToBlob(file.data_base64, file.mime_type),
  );
}
