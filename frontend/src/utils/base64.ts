export function base64ToBytes(dataBase64: string): Uint8Array<ArrayBuffer> {
  const binary = globalThis.atob(dataBase64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}
