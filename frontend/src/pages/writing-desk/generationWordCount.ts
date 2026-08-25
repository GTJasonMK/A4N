export interface GenerationWordCountRange {
  min: number;
  max: number;
}

export interface GenerationWordCountLabels {
  min: string;
  max: string;
  required: string;
  positive: string;
  minGreaterMax: string;
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
  labels: GenerationWordCountLabels,
): { value: number | null; error: string | null } => {
  const text = raw.trim();
  if (!text) return { value: null, error: `${label}${labels.required}` };
  if (!/^\d+$/.test(text)) {
    return { value: null, error: `${label}${labels.positive}` };
  }
  const value = Number(text);
  if (!Number.isSafeInteger(value) || value <= 0) {
    return { value: null, error: `${label}${labels.positive}` };
  }
  return { value, error: null };
};

export const parseGenerationWordCountRange = (
  minInput: string,
  maxInput: string,
  labels: GenerationWordCountLabels,
): { range: GenerationWordCountRange | null; error: string | null } => {
  const min = parsePositiveInteger(minInput, labels.min, labels);
  if (min.error) return { range: null, error: min.error };
  const max = parsePositiveInteger(maxInput, labels.max, labels);
  if (max.error) return { range: null, error: max.error };
  if (max.value! < min.value!) {
    return { range: null, error: labels.minGreaterMax };
  }
  return {
    range: { min: min.value!, max: max.value! },
    error: null,
  };
};
