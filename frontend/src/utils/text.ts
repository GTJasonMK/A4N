export const nonEmptyText = (value?: string | null): string | null => {
  const text = value?.trim();
  return text ? text : null;
};

export const textInputValue = (value: string | null | undefined): string =>
  value == null ? "" : value;

const textEncoder = new TextEncoder();

export const utf8ByteLength = (value: string): number =>
  textEncoder.encode(value).byteLength;
