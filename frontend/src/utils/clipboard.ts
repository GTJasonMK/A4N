export async function copyTextToClipboard(text: string): Promise<void> {
  let clipboardError: unknown = null;
  if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return;
    } catch (err) {
      clipboardError = err;
    }
  }

  if (copyTextWithTextarea(text)) return;

  const detail =
    clipboardError instanceof Error && clipboardError.message.trim()
      ? `：${clipboardError.message.trim()}`
      : "";
  throw new Error(`写入剪贴板失败${detail}`);
}

function copyTextWithTextarea(text: string): boolean {
  if (typeof document === "undefined" || !document.body) return false;

  const activeElement =
    document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "true");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  textarea.style.top = "0";
  textarea.style.opacity = "0";

  document.body.appendChild(textarea);
  textarea.focus();
  textarea.select();
  textarea.setSelectionRange(0, textarea.value.length);

  try {
    return document.execCommand("copy");
  } catch {
    return false;
  } finally {
    document.body.removeChild(textarea);
    try {
      activeElement?.focus({ preventScroll: true });
    } catch {
      activeElement?.focus();
    }
  }
}
