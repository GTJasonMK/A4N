export const nonEmptyText = (value?: string | null): string | null => {
  const text = value?.trim();
  return text ? text : null;
};

export const isDistinctText = (
  value?: string | null,
  reference?: string | null,
): boolean => {
  const text = nonEmptyText(value);
  return text !== null && text !== nonEmptyText(reference);
};

export const firstDistinctText = (
  values: readonly (string | null | undefined)[],
  reference?: string | null,
): string | null => nonEmptyText(
  values.find((value) => isDistinctText(value, reference)),
);

export const textInputValue = (value: string | null | undefined): string =>
  value == null ? "" : value;

const textEncoder = new TextEncoder();

export const utf8ByteLength = (value: string): number =>
  textEncoder.encode(value).byteLength;
