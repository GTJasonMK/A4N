import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useMemo } from "react";
import {
  Image,
  Link2,
} from "lucide-react";
import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type { BindMangaReferenceImageInput } from "../../../api/manga";
import type { Portrait } from "../../../api/portrait";
import { BookButton } from "../../../components/ui/BookButton";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { useI18n } from "../../../i18n";
import {
  buildReferenceCandidates,
  mangaReferenceAssetKey,
  type MangaPagePrompt,
} from "./mangaPagePromptModel";

export const ReferenceBindingsPanel: React.FC<{
  page: MangaPagePrompt;
  profiles: CharacterProfile[];
  portraits: Portrait[];
  projectImages: GeneratedImage[];
  assetsLoading?: boolean;
  chapterLocked: boolean;
  selectedCandidateByCharacter: Record<string, string>;
  onSelectCandidate: (bindingKey: string, value: string) => void;
  onBindReferenceImage?: (
    input: Omit<BindMangaReferenceImageInput, "downstream_strategy">,
  ) => void | Promise<void>;
}> = ({
  page,
  profiles,
  portraits,
  projectImages,
  assetsLoading = false,
  chapterLocked,
  selectedCandidateByCharacter,
  onSelectCandidate,
  onBindReferenceImage,
}) => {
  const { t } = useI18n();
  const candidatesByCharacter = useMemo(
    () => new Map(
      page.referenceBindings.map((binding) => [
        binding.characterName,
        buildReferenceCandidates(binding, profiles, portraits, projectImages, t),
      ]),
    ),
    [page.referenceBindings, profiles, portraits, projectImages, t],
  );
  if (page.referenceBindings.length === 0) {
    return (
      <div className="border-l-2 border-book-border/70 pl-2 text-xs text-book-text-muted">
        {t("comic.prompt.noReferenceNeeds")}
      </div>
    );
  }

  return (
    <div className="divide-y divide-book-border/35 border-y border-book-border/35">
      {page.referenceBindings.map((binding) => {
        const candidates = candidatesByCharacter.get(binding.characterName) ?? [];
        const bindingKey = `${page.prompt.id}:${binding.characterName}`;
        const boundKey = binding.boundAsset
          ? mangaReferenceAssetKey(
              binding.boundAsset.asset_kind,
              binding.boundAsset.asset_id,
            )
          : "";
        const selectedKey = selectedCandidateByCharacter[bindingKey] ?? boundKey;
        const selected = candidates.find((candidate) => candidate.key === selectedKey);
        const boundSrc = toMediaSrc(binding.boundAsset?.image_path);
        const bindDisabled =
          chapterLocked ||
          assetsLoading ||
          !onBindReferenceImage ||
          !selected ||
          selectedKey === boundKey;

        return (
          <div
            key={binding.characterName}
            className="py-3"
          >
            <div className="flex items-start gap-2">
              <div className="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-xl border border-book-border/50 bg-book-bg-tertiary text-caption text-book-text-muted">
                {boundSrc ? (
                  <img
                    src={boundSrc}
                    alt={binding.characterName}
                    className="h-full w-full object-cover"
                  />
                ) : (
                  <Image size={16} />
                )}
              </div>
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-x-1.5 gap-y-0.5">
                  <span className="truncate text-xs font-semibold text-book-text-main">
                    {binding.characterName}
                  </span>
                  <span
                    className={`text-caption ${
                      binding.boundAsset ? "text-success" : "text-warning"
                    }`}
                  >
                    {binding.boundAsset
                      ? t("comic.prompt.bound")
                      : t("comic.prompt.pendingBinding")}
                  </span>
                </div>
              </div>
            </div>

            <div className="mt-2 grid grid-cols-[minmax(0,1fr)_auto] gap-1.5">
              <ThemedSelect
                value={selectedKey}
                onChange={(selectedValue) => onSelectCandidate(bindingKey, selectedValue)}
                className="pyqt-input h-8 min-w-0 !px-2 !py-1 text-xs"
                disabled={chapterLocked || assetsLoading || candidates.length === 0}
              >
                <option value="">
                  {assetsLoading
                    ? t("comic.prompt.loadingImageAssets")
                    : candidates.length === 0
                      ? t("comic.prompt.noImageAssets")
                      : t("comic.prompt.selectAsset")}
                </option>
                {candidates.map((candidate) => (
                  <option key={candidate.key} value={candidate.key}>
                    {candidate.label} · {candidate.subLabel}
                  </option>
                ))}
              </ThemedSelect>
              <BookButton
                size="sm"
                variant="secondary"
                disabled={bindDisabled}
                onClick={() => {
                  if (!selected || !onBindReferenceImage) return;
                  void onBindReferenceImage({
                    prompt_id: page.prompt.id,
                    character_name: binding.characterName,
                    asset_kind: selected.assetKind,
                    asset_id: selected.assetId,
                  });
                }}
              >
                <Link2 size={12} />
                {binding.boundAsset ? t("comic.prompt.rebind") : t("comic.prompt.bind")}
              </BookButton>
            </div>
          </div>
        );
      })}
    </div>
  );
};
