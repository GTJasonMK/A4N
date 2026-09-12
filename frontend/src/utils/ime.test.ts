import { expect, it } from "vitest";
import { isImeCompositionActive } from "./ime";

it("recognizes native, Safari and just-finished composition without blocking ordinary keys", () => {
  const key = { isComposing: false, keyCode: 13 } as KeyboardEvent;
  expect(isImeCompositionActive(key)).toBe(false);
  expect(isImeCompositionActive({ ...key, isComposing: true } as KeyboardEvent)).toBe(true);
  expect(isImeCompositionActive({ ...key, keyCode: 229 } as KeyboardEvent)).toBe(true);
  expect(isImeCompositionActive(key, true)).toBe(true);
  expect(isImeCompositionActive(key, false, Date.now())).toBe(true);
  expect(isImeCompositionActive(key, false, Date.now() - 100)).toBe(false);
});
