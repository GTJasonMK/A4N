import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ThemedSelect } from "./ThemedSelect";

describe("ThemedSelect value contract", () => {
  it("keeps an unknown controlled value visible without a native select", () => {
    const html = renderToStaticMarkup(
      <ThemedSelect value="missing" onChange={() => undefined} aria-label="模式">
        <option value="placeholder" disabled>请选择模式</option>
        <option value="available">可用模式</option>
      </ThemedSelect>,
    );

    expect(html).toContain(">missing</span>");
    expect(html).not.toContain("<select");
  });

  it("reads numeric options inside fragments and rejects missing option values", () => {
    const html = renderToStaticMarkup(
      <ThemedSelect value={2} onChange={() => undefined}>
        <><option value={1}>一</option><option value={2}>二</option></>
      </ThemedSelect>,
    );
    expect(html).toContain(">二</span>");
    expect(() => renderToStaticMarkup(
      <ThemedSelect value="missing" onChange={() => undefined}>
        <option>缺少值</option>
      </ThemedSelect>,
    )).toThrow("ThemedSelect options require a string or number value");
  });
});
