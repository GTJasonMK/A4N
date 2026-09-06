import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ThemedSelect } from "./ThemedSelect";

describe("ThemedSelect value contract", () => {
  it("keeps an invalid default value visible instead of silently replacing it", () => {
    const html = renderToStaticMarkup(
      <ThemedSelect defaultValue="missing" aria-label="模式">
        <option value="placeholder" disabled>请选择模式</option>
        <option value="available">可用模式</option>
      </ThemedSelect>,
    );

    expect(html).toContain(">missing</span>");
  });
});
