import React, { useId, useMemo, useState } from "react";
import { Image, Link2, LoaderCircle, Upload, X } from "lucide-react";
import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type {
  BindMangaReferenceImageInput,
} from "../../../api/manga";
import type { Portrait } from "../../../api/portrait";
import { BookButton } from "../../../components/ui/BookButton";
import { BookInput } from "../../../components/ui/BookInput";
import { ZoomableImage } from "../../../components/ui/ImageLightbox";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { useI18n } from "../../../i18n";
import { supportedImageAccept } from "../../../utils/imageUpload";
import { toMediaSrc } from "../../../utils/mediaSrc";
import {
  buildReferenceCandidates,
  mangaReferenceAssetKey,
  searchReferenceCandidates,
  type MangaPagePrompt,
  type ReferenceCandidate,
} from "./mangaPagePromptModel";

type ReferenceMutation = BindMangaReferenceImageInput;
const DIRECT_REFERENCE_OPTION_LIMIT = 100;
const SEARCH_REFERENCE_RESULT_LIMIT = 100;
const MIN_REFERENCE_SEARCH_LENGTH = 2;

export const ReferenceBindingsPanel: React.FC<{
  page: MangaPagePrompt;
  profiles: CharacterProfile[];
  portraits: Portrait[];
  projectImages: GeneratedImage[];
  assetsLoading?: boolean;
  chapterLocked: boolean;
  selectedCandidateByCharacter: Record<string, string>;
  onSelectCandidate: (bindingKey: string, value: string) => void;
  onBindReferenceImage?: (input: ReferenceMutation) => void | Promise<void>;
  onUploadReferenceImage?: (
    promptId: number,
    characterName: string,
    file: File,
  ) => Promise<void>;
  uploadingReferenceBindingKey?: string | null;
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
  onUploadReferenceImage,
  uploadingReferenceBindingKey,
}) => {
  const { t } = useI18n();
  const searchHintPrefix = useId();
  const candidateIdentityLabel = (
    candidate: Pick<ReferenceCandidate, "assetKind" | "assetId" | "panelId">,
  ): string =>
    candidate.assetKind === "character_portrait"
      ? t("comic.prompt.referenceCandidatePortraitIdentity", {
          id: candidate.assetId,
        })
      : [
          t("comic.prompt.referenceCandidateImageIdentity", {
            id: candidate.assetId,
          }),
          candidate.panelId
            ? t("comic.prompt.referenceCandidatePanelIdentity", {
                panelId: candidate.panelId,
              })
            : null,
        ].filter(Boolean).join(" · ");
  const [searchQueries, setSearchQueries] = useState<Record<string, string>>({});
  const useSearchPicker = portraits.length + projectImages.length > DIRECT_REFERENCE_OPTION_LIMIT;
  const candidatesByCharacter = useMemo(
    () => new Map(page.referenceBindings.map((binding) => {
      const candidates = useSearchPicker
        ? searchReferenceCandidates(
            binding,
            profiles,
            portraits,
            projectImages,
            searchQueries[binding.characterName] ?? "",
            t,
            SEARCH_REFERENCE_RESULT_LIMIT,
          )
        : (() => {
            const all = buildReferenceCandidates(binding, profiles, portraits, projectImages, t);
            return { candidates: all, totalCount: all.length };
          })();
      return [binding.characterName, candidates] as const;
    })),
    [page.referenceBindings, profiles, portraits, projectImages, searchQueries, t, useSearchPicker],
  );
  const portraitReferenceCount = page.referenceBindings.filter(
    (binding) => binding.systemAsset != null,
  ).length;

  if (page.referenceBindings.length === 0 && page.systemReferenceImages.length === 0) {
    return (
      <div className="border-l-2 border-book-border/70 pl-2 text-xs text-book-text-muted">
        {t("comic.prompt.noReferenceNeeds")}
      </div>
    );
  }

  return (
    <div className="divide-y divide-book-border/35">
      {page.systemReferenceImages.length > 0 ? (
        <section className="space-y-2 border-y border-book-border/35 py-3">
          <h4 className="text-xs font-medium text-book-text-main">
            {t("comic.prompt.discoveredReferences")}
          </h4>
          <div className="divide-y divide-book-border/30">
            {page.systemReferenceImages.map((reference, index) => {
              const imageSrc = toMediaSrc(reference.asset.image_path);
              const targetKind = {
                character: t("comic.prompt.referenceTarget.character"),
                scene: t("comic.prompt.referenceTarget.scene"),
                object: t("comic.prompt.referenceTarget.object"),
                style: t("comic.prompt.referenceTarget.style"),
              }[reference.target_kind];
              return (
                <div
                  key={`${reference.asset.asset_kind}:${reference.asset.asset_id}`}
                  className="grid min-w-0 grid-cols-[40px_minmax(0,1fr)] items-center gap-2 py-2"
                >
                  <div className="flex h-10 w-10 items-center justify-center overflow-hidden rounded-md border border-book-border/50 bg-book-bg-tertiary text-caption text-book-text-muted">
                    {imageSrc ? (
                      <ZoomableImage
                        src={imageSrc}
                        alt={reference.target_name}
                        className="h-full w-full object-cover"
                      />
                    ) : (
                      <Image size={14} />
                    )}
                  </div>
                  <div className="min-w-0">
                    <div className="truncate text-xs font-medium text-book-text-main">
                      {targetKind} · {reference.target_name}
                    </div>
                    <p className="line-clamp-2 text-caption text-book-text-sub">
                      {reference.usage || reference.description}
                    </p>
                    <span className="text-caption text-book-text-muted">
                      {t("comic.prompt.referenceNumber", {
                        number: portraitReferenceCount + index + 1,
                      })}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        </section>
      ) : null}
      {page.referenceBindings.map((binding, bindingIndex) => {
        const candidateSearch = candidatesByCharacter.get(binding.characterName) ?? {
          candidates: [],
          totalCount: 0,
        };
        const userCandidates = candidateSearch.candidates;
        const searchQuery = searchQueries[binding.characterName] ?? "";
        const searchHintId = `${searchHintPrefix}-manga-reference-search-hint-${bindingIndex}`;
        const bindingKey = `${page.prompt.id}:${binding.characterName}`;
        const userSelectionKey = `${bindingKey}:user`;
        const selectedUserKey = selectedCandidateByCharacter[userSelectionKey] ?? "";
        const selectedUser = userCandidates.find(
          (candidate) => candidate.key === selectedUserKey,
        );
        const visibleSelectedUserKey = selectedUser ? selectedUserKey : "";
        const selectedUserImageSrc = selectedUser
          ? toMediaSrc(selectedUser.imagePath)
          : null;
        const uploading = uploadingReferenceBindingKey === bindingKey;
        const mutationDisabled =
          chapterLocked || assetsLoading || uploadingReferenceBindingKey != null;
        const systemImageSrc = toMediaSrc(binding.systemAsset?.image_path);
        const handleSearchChange = (value: string) => {
          setSearchQueries((current) => ({
            ...current,
            [binding.characterName]: value,
          }));
          if (selectedCandidateByCharacter[userSelectionKey]) {
            onSelectCandidate(userSelectionKey, "");
          }
        };

        return (
          <article key={binding.characterName} className="space-y-3 py-3">
            <div className="flex min-w-0 items-center gap-2">
              <span className="truncate text-xs font-semibold text-book-text-main">
                {binding.characterName}
              </span>
              {binding.systemAsset ? (
                <span className="text-caption text-success">{t("comic.prompt.bound")}</span>
              ) : binding.userAddedAssets.length > 0 ? (
                <span className="text-caption text-warning">
                  {t("comic.prompt.userReferenceOnly")}
                </span>
              ) : null}
            </div>

            <section className="space-y-2 rounded-md border border-book-border/40 bg-book-bg-paper/35 p-2">
              <h4 className="text-xs font-medium text-book-text-sub">
                {t("comic.prompt.systemReferences")}
              </h4>
              <div className="grid min-w-0 grid-cols-[48px_minmax(0,1fr)] items-center gap-2">
                <div className="flex h-12 w-12 items-center justify-center overflow-hidden rounded-md border border-book-border/50 bg-book-bg-tertiary text-caption text-book-text-muted">
                  {systemImageSrc ? (
                    <ZoomableImage
                      src={systemImageSrc}
                      alt={binding.characterName}
                      className="h-full w-full object-cover"
                    />
                  ) : (
                    <Image size={16} />
                  )}
                </div>
                <span className="min-w-0 text-xs text-book-text-sub">
                  {binding.systemAsset
                    ? t("comic.prompt.systemAutoSelected")
                    : t("comic.prompt.noSystemAutoSelected")}
                </span>
              </div>
            </section>

            <section className="space-y-2 rounded-md border border-book-border/40 bg-book-bg-paper/35 p-2">
              <div className="flex min-w-0 items-center justify-between gap-2">
                <h4 className="text-xs font-medium text-book-text-sub">
                  {t("comic.prompt.userReferences")}
                </h4>
                <span className="shrink-0 text-caption tabular-nums text-book-text-muted">
                  {binding.userAddedAssets.length}
                </span>
              </div>

              {binding.userAddedAssets.length > 0 ? (
                <div className="grid gap-1.5">
                  {binding.userAddedAssets.map((asset, index) => {
                    const assetKey = mangaReferenceAssetKey(
                      asset.asset_kind,
                      asset.asset_id,
                    );
                    const candidate = userCandidates.find(
                      (item) => item.key === assetKey,
                    );
                    const projectImage = projectImages.find(
                      (image) => image.id === asset.asset_id,
                    );
                    const imageSrc = toMediaSrc(
                      candidate?.imagePath ?? asset.image_path,
                    );
                    const assetLabel =
                      projectImage?.prompt.trim() ||
                      `${candidate?.label ?? t("comic.prompt.projectImage")} · ${asset.asset_id}`;
                    return (
                      <div
                        key={assetKey}
                        className="grid min-w-0 grid-cols-[40px_minmax(0,1fr)_32px] items-center gap-2"
                      >
                        <div className="flex h-10 w-10 items-center justify-center overflow-hidden rounded-md border border-book-border/50 bg-book-bg-tertiary text-caption text-book-text-muted">
                          {imageSrc ? (
                            <ZoomableImage
                              src={imageSrc}
                              alt={`${binding.characterName} ${t("comic.prompt.userReferences")} ${index + 1}`}
                              className="h-full w-full object-cover"
                            />
                          ) : (
                            <Image size={14} />
                          )}
                        </div>
                        <span
                          className="min-w-0 truncate text-xs text-book-text-main"
                          title={assetLabel}
                        >
                          {assetLabel}
                        </span>
                        <BookButton
                          size="sm"
                          variant="ghost"
                          className="!h-8 !w-8 !min-w-8 !p-0"
                          aria-label={t("comic.prompt.removeUserReference", {
                            character: binding.characterName,
                            index: index + 1,
                          })}
                          disabled={mutationDisabled || !onBindReferenceImage}
                          onClick={() =>
                            void onBindReferenceImage?.({
                              action: "remove_user_image",
                              prompt_id: page.prompt.id,
                              character_name: binding.characterName,
                              asset_kind: asset.asset_kind,
                              asset_id: asset.asset_id,
                            })
                          }
                        >
                          <X size={13} />
                        </BookButton>
                      </div>
                    );
                  })}
                </div>
              ) : (
                <div className="text-caption text-book-text-muted">
                  {t("comic.prompt.noUserReferences")}
                </div>
              )}

              {useSearchPicker ? (
                <div className="space-y-1.5">
                  <BookInput
                    label={t("comic.prompt.searchExistingUserReference")}
                    placeholder={t("comic.prompt.searchExistingUserReferencePlaceholder")}
                    value={searchQuery}
                    onChange={(event) => handleSearchChange(event.currentTarget.value)}
                    disabled={mutationDisabled}
                    aria-describedby={searchHintId}
                  />
                  <p
                    id={searchHintId}
                    role="status"
                    aria-live="polite"
                    className="min-h-4 truncate text-caption text-book-text-muted"
                  >
                    {assetsLoading
                      ? t("comic.prompt.loadingImageAssets")
                      : searchQuery.trim().length < MIN_REFERENCE_SEARCH_LENGTH
                        ? t("comic.prompt.searchExistingUserReferenceHint")
                        : candidateSearch.totalCount === 0
                          ? t("comic.prompt.noMatchingReferenceCandidates")
                          : candidateSearch.totalCount > userCandidates.length
                            ? t("comic.prompt.referenceCandidateTruncated", {
                                total: candidateSearch.totalCount,
                                shown: userCandidates.length,
                              })
                            : t("comic.prompt.referenceCandidateCount", {
                                total: candidateSearch.totalCount,
                              })}
                  </p>
                </div>
              ) : null}

              <div className="grid gap-1.5 sm:grid-cols-[minmax(0,1fr)_auto_auto]">
                <div className="grid min-w-0 gap-1.5">
                  <ThemedSelect
                    aria-label={`${binding.characterName} · ${t("comic.prompt.addExistingUserReference")}`}
                    value={visibleSelectedUserKey}
                    onChange={(value) => onSelectCandidate(userSelectionKey, value)}
                    className="pyqt-input h-8 min-w-0 !px-2 !py-1 text-xs"
                    disabled={mutationDisabled || userCandidates.length === 0}
                  >
                    <option value="" disabled={useSearchPicker && userCandidates.length > 0}>
                      {assetsLoading
                        ? t("comic.prompt.loadingImageAssets")
                        : useSearchPicker
                          ? t("comic.prompt.selectReferenceCandidate")
                          : userCandidates.length === 0
                            ? t("comic.prompt.noUserReferenceCandidates")
                            : t("comic.prompt.addExistingUserReference")}
                    </option>
                    {userCandidates.map((candidate) => (
                      <option key={candidate.key} value={candidate.key}>
                        {candidate.label} · {candidate.subLabel}
                        {useSearchPicker
                          ? " · " + candidateIdentityLabel(candidate) +
                            (candidate.searchPreview ? " · " + candidate.searchPreview : "")
                          : null}
                      </option>
                    ))}
                  </ThemedSelect>
                  {selectedUser ? (
                    <div className="flex min-w-0 items-center gap-2 rounded-md border border-book-border/40 bg-book-bg-paper/50 p-1.5">
                      <div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-md border border-book-border/50 bg-book-bg-tertiary text-caption text-book-text-muted">
                        {selectedUserImageSrc ? (
                          <ZoomableImage
                            src={selectedUserImageSrc}
                            alt={`${selectedUser.label} · ${candidateIdentityLabel(selectedUser)}`}
                            className="h-full w-full object-cover"
                          />
                        ) : (
                          <Image size={14} />
                        )}
                      </div>
                      <div className="min-w-0">
                        <div className="truncate text-xs font-medium text-book-text-main">
                          {selectedUser.label} · {candidateIdentityLabel(selectedUser)}
                        </div>
                        <p className="line-clamp-2 text-caption text-book-text-muted">
                          {selectedUser.searchPreview}
                        </p>
                      </div>
                    </div>
                  ) : null}
                </div>
                <BookButton
                  size="sm"
                  variant="secondary"
                  disabled={
                    mutationDisabled ||
                    !onBindReferenceImage ||
                    !selectedUser ||
                    (binding.systemAsset?.asset_kind === selectedUser.assetKind &&
                      binding.systemAsset.asset_id === selectedUser.assetId) ||
                    binding.userAddedAssets.some(
                      (asset) =>
                        asset.asset_kind === selectedUser.assetKind &&
                        asset.asset_id === selectedUser.assetId,
                    )
                  }
                  onClick={() => {
                    if (!selectedUser || !onBindReferenceImage) return;
                    void onBindReferenceImage({
                      action: "add_user_image",
                      prompt_id: page.prompt.id,
                      character_name: binding.characterName,
                      asset_kind: selectedUser.assetKind,
                      asset_id: selectedUser.assetId,
                    });
                  }}
                >
                  <Link2 size={12} />
                  {t("comic.prompt.addReference")}
                </BookButton>
                {onUploadReferenceImage ? (
                  <label
                    className={[
                      "relative inline-flex min-h-8 w-full items-center justify-center gap-1 rounded-md border px-2 text-xs font-medium transition-colors focus-within:outline focus-within:outline-1 focus-within:outline-book-primary sm:w-auto",
                      mutationDisabled
                        ? "cursor-not-allowed border-book-border/40 text-book-text-muted opacity-50"
                        : "cursor-pointer border-book-border/60 bg-book-bg-paper/70 text-book-text-sub hover:border-book-primary/60 hover:text-book-text-main",
                    ].join(" ")}
                  >
                    {uploading ? (
                      <LoaderCircle size={12} className="animate-spin" />
                    ) : (
                      <Upload size={12} />
                    )}
                    {uploading
                      ? t("comic.prompt.uploadingReference")
                      : t("comic.prompt.uploadReference")}
                    <input
                      type="file"
                      accept={supportedImageAccept()}
                      aria-label={`${binding.characterName} · ${t("comic.prompt.uploadReference")}`}
                      disabled={mutationDisabled}
                      className="sr-only"
                      onChange={(event) => {
                        const input = event.currentTarget;
                        const file = input.files?.[0];
                        input.value = "";
                        if (file) {
                          void onUploadReferenceImage(
                            page.prompt.id,
                            binding.characterName,
                            file,
                          );
                        }
                      }}
                    />
                  </label>
                ) : null}
              </div>
            </section>
          </article>
        );
      })}
    </div>
  );
};
