import { expect, it } from "vitest";
import type { MessageTranslator } from "../../i18n";
import {
  CHARACTER_DETAIL_DRAFT_SECTIONS,
  attributeKeyLabel,
  characterDetailDraftKey,
  parsePositiveChapterNumber,
} from "./manageViewUtils";

it("parses only safe positive whole chapter numbers", () => {
  expect(parsePositiveChapterNumber("12")).toBe(12);
  expect(parsePositiveChapterNumber("")).toBeNull();
  expect(parsePositiveChapterNumber("0")).toBeNull();
  expect(parsePositiveChapterNumber("2.5")).toBeNull();
  expect(parsePositiveChapterNumber("2147483648")).toBeNull();
  expect(parsePositiveChapterNumber("9007199254740992")).toBeNull();
});

it("keeps custom attribute identities distinct in records and deletion confirmations", () => {
  const t: MessageTranslator = (key) => key;
  expect(attributeKeyLabel("temporary_item", t)).toBe("Temporary Item");
  expect(attributeKeyLabel("avoidance", t)).toBe("Avoidance");
  expect(attributeKeyLabel("current_location", t)).toBe("characterDetail.labels.currentLocation");
  expect(attributeKeyLabel("临时物品", t)).toBe("临时物品");
  expect(attributeKeyLabel("", t)).toBe("characterDetail.labels.unnamedAttribute");
});

it("keeps character detail operation keys scoped to one profile and section", () => {
  expect(CHARACTER_DETAIL_DRAFT_SECTIONS).toEqual(["event", "deletion", "implicit", "snapshot"]);
  expect(characterDetailDraftKey(7, "event")).toBe("character-detail:7:event");
  expect(characterDetailDraftKey(8, "event")).not.toBe(characterDetailDraftKey(7, "event"));
  expect(characterDetailDraftKey(7, "snapshot")).toBe("character-detail:7:snapshot");
});
