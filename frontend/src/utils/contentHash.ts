export async function sha256Hex(text: string): Promise<string | null> {
  if (!window.crypto?.subtle) return null;
  const digest = await window.crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return Array.from(new Uint8Array(digest))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}
