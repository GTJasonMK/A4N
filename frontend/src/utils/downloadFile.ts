interface DownloadableBase64File {
  file_name: string;
  mime_type?: string | null;
  data_base64: string;
}

function base64DataToBlob(
  dataBase64: string,
  mimeType = "application/octet-stream",
): Blob {
  const parts = dataBase64.split(";base64,");
  const raw = (parts[parts.length - 1] ?? "").trim();
  const binary = window.atob(raw);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
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

export function downloadBase64File(
  fileName: string,
  dataBase64: string,
  mimeType = "application/octet-stream",
): void {
  downloadBlob(fileName, base64DataToBlob(dataBase64, mimeType));
}

export function downloadBase64ExportFile(file: DownloadableBase64File): void {
  downloadBase64File(
    file.file_name,
    file.data_base64,
    file.mime_type || "application/octet-stream",
  );
}
