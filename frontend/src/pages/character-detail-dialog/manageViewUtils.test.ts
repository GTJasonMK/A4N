import { expect, it } from "vitest";
import type { MessageTranslator } from "../../i18n";
import { attributeKeyLabel } from "./manageViewUtils";

it("keeps custom attribute identities distinct in records and deletion confirmations", () => {
  const t: MessageTranslator = (key) => key;
  expect(attributeKeyLabel("temporary_item", t)).toBe("temporary_item");
  expect(attributeKeyLabel("avoidance", t)).toBe("avoidance");
  expect(attributeKeyLabel("", t)).toBe("characterDetail.labels.unnamedAttribute");
});
