import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ThemedCombobox } from "./ThemedCombobox";

describe("ThemedCombobox accessibility contract", () => {
  it("renders a themed free-input combobox instead of a native datalist", () => {
    const html = renderToStaticMarkup(
      <label>
        <span>模型</span>
        <ThemedCombobox
          value="model-a"
          options={[{ value: "model-a", label: "model-a · local" }]}
          onChange={() => undefined}
          placeholder="选择或输入模型"
        />
      </label>,
    );

    expect(html).toContain('role="combobox"');
    expect(html).toContain('aria-autocomplete="list"');
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain("a4n-themed-combobox__input");
    expect(html).not.toContain("list=");
  });
});
