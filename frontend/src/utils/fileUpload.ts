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

export function openFilePicker(input: HTMLInputElement | null): void {
  if (!input || input.disabled) return;
  if (typeof input.showPicker === "function") {
    try {
      input.showPicker();
      return;
    } catch {
      // Browsers can reject showPicker outside a user activation; click is the compatible fallback.
    }
  }
  input.click();
}

export function pickLocalFile(accept: string): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    let finished = false;
    const finish = (file: File | null) => {
      if (finished) return;
      finished = true;
      input.remove();
      resolve(file);
    };
    input.type = "file";
    input.accept = accept;
    input.multiple = false;
    input.style.position = "fixed";
    input.style.width = "1px";
    input.style.height = "1px";
    input.style.opacity = "0";
    input.style.pointerEvents = "none";
    input.style.top = "-1px";
    input.onchange = () => {
      finish(input.files?.[0] ?? null);
    };
    input.addEventListener("cancel", () => finish(null));
    document.body.appendChild(input);
    openFilePicker(input);
  });
}
