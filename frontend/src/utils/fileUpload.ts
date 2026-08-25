import { getActiveLocale, translateMessage } from "../i18n";

export function readFileAsBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error(translateMessage(getActiveLocale(), "errors.fileReadFailed")));
    reader.onload = () => {
      if (typeof reader.result !== "string") {
        reject(new Error(translateMessage(getActiveLocale(), "errors.fileReadFailed")));
        return;
      }
      const [, body] = reader.result.split(",", 2);
      if (!body) {
        reject(new Error(translateMessage(getActiveLocale(), "errors.invalidFileData")));
        return;
      }
      resolve(body);
    };
    reader.readAsDataURL(file);
  });
}

export function pickLocalFile(accept: string): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    let resolved = false;
    let focusTimer: number | null = null;
    const cleanup = () => {
      if (focusTimer != null) {
        window.clearTimeout(focusTimer);
        focusTimer = null;
      }
      window.removeEventListener("focus", handleFocus);
      input.remove();
    };
    const finish = (file: File | null) => {
      if (resolved) return;
      resolved = true;
      cleanup();
      resolve(file);
    };
    const handleFocus = () => {
      focusTimer = window.setTimeout(() => {
        finish(input.files?.[0] ?? null);
      }, 250);
    };
    input.type = "file";
    input.accept = accept;
    input.multiple = false;
    input.style.display = "none";
    input.onchange = () => {
      finish(input.files?.[0] ?? null);
    };
    document.body.appendChild(input);
    window.addEventListener("focus", handleFocus);
    input.click();
  });
}
