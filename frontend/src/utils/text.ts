export const nonEmptyText = (value?: string | null): string | null => {
  const text = value?.trim();
  return text ? text : null;
};

export const textInputValue = (value: string | null | undefined): string =>
  value == null ? "" : value;
