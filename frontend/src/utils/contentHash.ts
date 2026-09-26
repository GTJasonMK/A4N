export async function sha256Hex(text: string): Promise<string | null> {
  if (!globalThis.crypto?.subtle) return null;
  const digest = await globalThis.crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return Array.from(new Uint8Array(digest))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

export function sha256Json(value: unknown): Promise<string | null> {
  return sha256Hex(canonicalJson(value));
}

/** Match the backend's RFC 8785 ordering and JavaScript number serialization. */
export function canonicalJson(value: unknown): string {
  // Normalize to JSON first so optional fields, arrays and toJSON keep their JSON semantics.
  return encodeCanonicalJson(JSON.parse(JSON.stringify(value)));
}

function encodeCanonicalJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(encodeCanonicalJson).join(",")}]`;
  if (value !== null && typeof value === "object") {
    const object = value as Record<string, unknown>;
    return `{${Object.keys(object).sort().map((key) =>
      `${JSON.stringify(key)}:${encodeCanonicalJson(object[key])}`,
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}
