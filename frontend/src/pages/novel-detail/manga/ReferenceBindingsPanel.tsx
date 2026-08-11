import React from "react";
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
import {
  buildReferenceCandidates,
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
  if (page.referenceBindings.length === 0) {
    return (
      <div className="border-l-2 border-book-border/70 pl-2 text-xs text-book-text-muted">
        本页没有角色参考图需求。
      </div>
    );
  }

  return (
    <div className="divide-y divide-book-border/35 border-y border-book-border/35">
      {page.referenceBindings.map((binding) => {
        const candidates = buildReferenceCandidates(binding, profiles, portraits, projectImages);
        const bindingKey = `${page.prompt.id}:${binding.characterName}`;
        const boundKey = binding.boundAsset
          ? `${binding.boundAsset.asset_kind}:${binding.boundAsset.asset_id}`
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
                    {binding.boundAsset ? "已绑定" : "待绑定"}
                  </span>
                </div>
              </div>
            </div>

            <div className="mt-2 grid grid-cols-[minmax(0,1fr)_auto] gap-1.5">
              <select
                value={selectedKey}
                onChange={(event) => onSelectCandidate(bindingKey, event.target.value)}
                className="pyqt-input h-8 min-w-0 !px-2 !py-1 text-xs"
                disabled={chapterLocked || assetsLoading || candidates.length === 0}
              >
                <option value="">
                  {assetsLoading
                    ? "加载素材…"
                    : candidates.length === 0
                      ? "无可用素材"
                      : "选择素材"}
                </option>
                {candidates.map((candidate) => (
                  <option key={candidate.key} value={candidate.key}>
                    {candidate.label} · {candidate.subLabel}
                  </option>
                ))}
              </select>
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
                {binding.boundAsset ? "改绑" : "绑定"}
              </BookButton>
            </div>
          </div>
        );
      })}
    </div>
  );
};
