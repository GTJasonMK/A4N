import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";

export const trimmedText = (value: string | null | undefined): string =>
  value == null ? "" : value.trim();

export const positiveIntegerOrNull = (value: unknown): number | null =>
  typeof value === "number" && Number.isInteger(value) && value > 0
    ? value
    : null;

export const extractSourceContentHash = (metadata: unknown): string | null => {
  const source = asRecord(metadata)?.source;
  const hash = asRecord(source)?.content_sha256;
  return typeof hash === "string" && hash.length > 0 && hash === hash.trim()
    ? hash
    : null;
};
