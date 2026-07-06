export interface GenerationWordCountRange {
  min: number;
  max: number;
}

export const DEFAULT_GENERATION_WORD_COUNT_RANGE: GenerationWordCountRange = {
  min: 4500,
  max: 6000,
};

export const defaultGenerationWordCountRange = (
  target: number | null | undefined,
): GenerationWordCountRange => {
  if (typeof target !== "number" || !Number.isFinite(target) || target <= 0) {
    return DEFAULT_GENERATION_WORD_COUNT_RANGE;
  }
  const normalizedTarget = Math.floor(target);
  const min = Math.max(1, Math.floor(normalizedTarget * 0.9));
  const max = Math.max(min, Math.ceil(normalizedTarget * 1.1));
  return { min, max };
};

const parsePositiveInteger = (
  raw: string,
  label: string,
): { value: number | null; error: string | null } => {
  const text = raw.trim();
  if (!text) return { value: null, error: `${label}不能为空` };
  if (!/^\d+$/.test(text)) {
    return { value: null, error: `${label}必须是正整数` };
  }
  const value = Number(text);
  if (!Number.isSafeInteger(value) || value <= 0) {
    return { value: null, error: `${label}必须是正整数` };
  }
  return { value, error: null };
};

export const parseGenerationWordCountRange = (
  minInput: string,
  maxInput: string,
): { range: GenerationWordCountRange | null; error: string | null } => {
  const min = parsePositiveInteger(minInput, "最少字数");
  if (min.error) return { range: null, error: min.error };
  const max = parsePositiveInteger(maxInput, "最多字数");
  if (max.error) return { range: null, error: max.error };
  if (max.value! < min.value!) {
    return { range: null, error: "最多字数不能小于最少字数" };
  }
  return {
    range: { min: min.value!, max: max.value! },
    error: null,
  };
};
