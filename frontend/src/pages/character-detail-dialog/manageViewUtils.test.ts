import { expect, it } from "vitest";
import type { MessageTranslator } from "../../i18n";
import {
  CHARACTER_DETAIL_DRAFT_SECTIONS,
  attributeKeyLabel,
  characterDetailDraftKey,
} from "./manageViewUtils";

it("keeps custom attribute identities distinct in records and deletion confirmations", () => {
  const t: MessageTranslator = (key) => key;
  expect(attributeKeyLabel("temporary_item", t)).toBe("temporary_item");
  expect(attributeKeyLabel("avoidance", t)).toBe("avoidance");
  expect(attributeKeyLabel("", t)).toBe("characterDetail.labels.unnamedAttribute");
});

it("keeps character detail draft keys scoped to one profile and section", () => {
  expect(CHARACTER_DETAIL_DRAFT_SECTIONS).toEqual(["event", "deletion", "implicit"]);
  expect(characterDetailDraftKey(7, "event")).toBe("character-detail:7:event");
  expect(characterDetailDraftKey(8, "event")).not.toBe(characterDetailDraftKey(7, "event"));
});
