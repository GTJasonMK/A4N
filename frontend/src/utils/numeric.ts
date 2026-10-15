export const MAX_I32 = 2_147_483_647;

export const isPositiveI32 = (value: number): boolean =>
  Number.isSafeInteger(value) && value > 0 && value <= MAX_I32;
