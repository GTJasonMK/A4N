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
  selectedCandidateBySlot: Record<string, string>;
  onSelectCandidate: (slotKey: string, value: string) => void;
  onBindReferenceImage?: (input: BindMangaReferenceImageInput) => void | Promise<void>;
}> = ({
  page,
  profiles,
  portraits,
  projectImages,
  assetsLoading = false,
  chapterLocked,
  selectedCandidateBySlot,
  onSelectCandidate,
  onBindReferenceImage,
}) => {
  if (page.referenceBindings.length === 0) {
    return (
      <div className="border-l-2 border-book-border/70 pl-2 text-xs text-book-text-muted">
        本页没有参考图槽位。
      </div>
    );
  }

  return (
    <div className="divide-y divide-book-border/35 border-y border-book-border/35">
      {page.referenceBindings.map((slot) => {
        const candidates = buildReferenceCandidates(slot, profiles, portraits, projectImages);
        const slotKey = `${page.prompt.id}:${slot.slotId}`;
        const boundKey = slot.boundAsset
          ? `${slot.boundAsset.asset_kind}:${slot.boundAsset.asset_id}`
          : "";
        const selectedKey = selectedCandidateBySlot[slotKey] ?? boundKey ?? candidates[0]?.key ?? "";
        const selected = candidates.find((candidate) => candidate.key === selectedKey);
        const boundSrc = toMediaSrc(slot.boundAsset?.image_path);
        const bindDisabled =
          chapterLocked ||
          assetsLoading ||
          !onBindReferenceImage ||
          !selected ||
          selectedKey === boundKey;

        return (
          <div
            key={slot.slotId}
            className="py-3"
          >
            <div className="flex items-start gap-2">
              <div className="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-xl border border-book-border/50 bg-book-bg-tertiary text-[10px] text-book-text-muted">
                {boundSrc ? (
                  <img src={boundSrc} alt={slot.label} className="h-full w-full object-cover" />
                ) : (
                  <Image size={16} />
                )}
              </div>
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-x-1.5 gap-y-0.5">
                  <span className="truncate text-xs font-semibold text-book-text-main">
                    {slot.label}
                  </span>
                  <span
                    className={`text-[10px] ${
                      slot.boundAsset
                        ? "text-success"
                        : slot.required
                          ? "text-warning"
                          : "text-book-text-muted"
                    }`}
                  >
                    {slot.boundAsset ? "已绑定" : slot.required ? "必需" : "可选"}
                  </span>
                </div>
                <div className="mt-0.5 line-clamp-2 text-[11px] leading-relaxed text-book-text-muted">
                  {slot.purpose ?? slot.targetName ?? slot.slotId}
                </div>
              </div>
            </div>

            <div className="mt-2 grid grid-cols-[minmax(0,1fr)_auto] gap-1.5">
              <select
                value={selectedKey}
                onChange={(event) => onSelectCandidate(slotKey, event.target.value)}
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
                    slot_id: slot.slotId,
                    asset_kind: selected.assetKind,
                    asset_id: selected.assetId,
                  });
                }}
              >
                <Link2 size={12} />
                {slot.boundAsset ? "改绑" : "绑定"}
              </BookButton>
            </div>
          </div>
        );
      })}
    </div>
  );
};
