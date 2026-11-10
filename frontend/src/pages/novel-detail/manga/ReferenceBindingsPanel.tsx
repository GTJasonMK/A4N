import React, { useId, useMemo, useState } from "react";
import { Image, ImagePlus, Link2, LoaderCircle, Upload, X } from "lucide-react";
import type { CharacterProfile } from "../../../api/character";
import type { GeneratedImage } from "../../../api/image";
import type {
  BindMangaReferenceImageInput,
  MangaReferenceAssetKind,
  MangaPageReferenceTargetKind,
} from "../../../api/manga";
import type { Portrait } from "../../../api/portrait";
import { MANGA_PAGE_REFERENCE_IMAGE_LIMIT, MANGA_REFERENCE_IMAGES_PER_TARGET } from "../../../api/generated/config";
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
const REFERENCE_IMAGE_PAGE_SIZE = 8;
const MIN_REFERENCE_SEARCH_LENGTH = 2;
type ReferenceTarget = {
  targetKind: MangaPageReferenceTargetKind;
  targetName: string;
  description?: string;
  usage?: string;
};

const referenceTargetKey = (target: ReferenceTarget, index: number): string =>
  `${target.targetKind}:${target.targetName}:${index}`;

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
    input: ReferenceMutation,
    sourceImage?: GeneratedImage,
  ) => void | Promise<void>;
  onBindDiscoveredReference?: (
    sourceImage: GeneratedImage | File,
    target: {
      targetKind: MangaPageReferenceTargetKind;
      targetName: string;
    },
  ) => void | Promise<void>;
  onGenerateDiscoveredReference?: (
    promptId: number,
    target: { targetKind: MangaPageReferenceTargetKind; targetName: string },
  ) => Promise<void>;
  onUnbindDiscoveredReference?: (
    assetKind: MangaReferenceAssetKind,
    assetId: number,
  ) => void | Promise<void>;
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
  onBindDiscoveredReference,
  onGenerateDiscoveredReference,
  onUnbindDiscoveredReference,
  onUploadReferenceImage,
  uploadingReferenceBindingKey,
}) => {
  const { t } = useI18n();
  const searchHintPrefix = useId();
  const [referenceActionError, setReferenceActionError] = useState<string | null>(null);
  const [referenceImageQuery, setReferenceImageQuery] = useState("");
  const [visibleImageCounts, setVisibleImageCounts] = useState<Record<string, number>>({});
  const [bindingReferenceTargetKey, setBindingReferenceTargetKey] = useState<string | null>(null);
  const [addingReferenceTargetKey, setAddingReferenceTargetKey] = useState<string | null>(null);
  const [generatingReferenceTargetKey, setGeneratingReferenceTargetKey] = useState<string | null>(null);
  const referenceRequirements = page.referenceDiscovery?.requirements ?? [];
  const discoveredReferences = page.referenceDiscovery?.references ?? [];
  const referenceTargetKindLabel = (targetKind: MangaPageReferenceTargetKind): string => ({
    character: t("comic.prompt.referenceTarget.character"),
    scene: t("comic.prompt.referenceTarget.scene"),
    object: t("comic.prompt.referenceTarget.object"),
    style: t("comic.prompt.referenceTarget.style"),
  })[targetKind];
  const systemPortraitBindings = useMemo(
    () => page.referenceBindings.filter((binding) => binding.systemAsset != null),
    [page.referenceBindings],
  );
  const portraitReferenceCount = systemPortraitBindings.length;
  const referenceTargets = useMemo<ReferenceTarget[]>(() => {
    const targets: ReferenceTarget[] = referenceRequirements.map((requirement) => ({
      targetKind: requirement.target_kind,
      targetName: requirement.target_name,
      description: requirement.description,
      usage: requirement.usage,
    }));
    for (const reference of discoveredReferences) {
      if (targets.some((target) =>
        target.targetKind === reference.target_kind && target.targetName === reference.target_name,
      )) continue;
      targets.push({
        targetKind: reference.target_kind,
        targetName: reference.target_name,
        description: reference.description,
        usage: reference.usage,
      });
    }
    for (const binding of systemPortraitBindings) {
      if (targets.some((target) =>
        target.targetKind === "character" && target.targetName === binding.characterName,
      )) continue;
      targets.push({
        targetKind: "character",
        targetName: binding.characterName,
      });
    }
    return targets;
  }, [discoveredReferences, referenceRequirements, systemPortraitBindings]);
  const canEditReferenceDiscovery =
    page.referenceDiscovery != null && referenceTargets.length > 0 &&
    onBindDiscoveredReference != null;
  const referenceImages = projectImages;
  const visibleReferenceImages = useMemo(() => {
    const query = referenceImageQuery.trim().toLocaleLowerCase();
    return referenceImages
      .filter((image) => {
        if (!query) return true;
        return [
          image.prompt,
          image.image_type,
          image.panel_id,
          image.file_path,
          JSON.stringify(image.metadata_json ?? {}),
          String(image.id),
        ].filter(Boolean).join(" ").toLocaleLowerCase().includes(query);
      })
      .sort((a, b) => b.created_at.localeCompare(a.created_at));
  }, [referenceImageQuery, referenceImages]);
  const referenceImagesByTarget = useMemo(() => new Map(
    referenceTargets.map((target, index) => {
      const targetTerms = [target.targetName, target.description, target.usage]
        .filter((term): term is string => Boolean(term))
        .map((term) => term.trim().toLocaleLowerCase());
      const score = (image: GeneratedImage) => {
        const searchText = [
          image.prompt,
          image.image_type,
          image.panel_id,
          JSON.stringify(image.metadata_json ?? {}),
        ].filter(Boolean).join(" ").toLocaleLowerCase();
        return targetTerms.reduce(
          (total, term, termIndex) =>
            total + (searchText.includes(term) ? (termIndex === 0 ? 100 : 20) : 0),
          0,
        );
      };
      const images = [...visibleReferenceImages].sort((a, b) =>
        score(b) - score(a) || b.created_at.localeCompare(a.created_at),
      );
      return [referenceTargetKey(target, index), {
        matches: images.filter((image) => score(image) > 0),
        all: images,
      }] as const;
    }),
  ), [referenceTargets, visibleReferenceImages]);
  const referenceImageById = useMemo(
    () => new Map(referenceImages.map((image) => [image.id, image])),
    [referenceImages],
  );
  const unavailableSystemReferences = assetsLoading
    ? []
    : discoveredReferences.filter(
        (reference) =>
          reference.asset.asset_kind !== "generated_image" ||
          !referenceImageById.has(reference.asset.asset_id),
      );
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
  const useSearchPicker = portraits.length + referenceImages.length > DIRECT_REFERENCE_OPTION_LIMIT;
  const candidatesByCharacter = useMemo(
    () => new Map(page.referenceBindings.map((binding) => {
      const candidates = useSearchPicker
        ? searchReferenceCandidates(
            binding,
            profiles,
            portraits,
            referenceImages,
            searchQueries[binding.characterName] ?? "",
            t,
            SEARCH_REFERENCE_RESULT_LIMIT,
          )
        : (() => {
            const all = buildReferenceCandidates(binding, profiles, portraits, referenceImages, t);
            return { candidates: all, totalCount: all.length };
          })();
      return [binding.characterName, candidates] as const;
    })),
    [page.referenceBindings, profiles, portraits, referenceImages, searchQueries, t, useSearchPicker],
  );
  const bindDiscoveredReference = async (
    image: GeneratedImage | File,
    target: ReferenceTarget,
    targetKey: string,
  ) => {
    if (!onBindDiscoveredReference || addingReferenceTargetKey != null) return;
    setAddingReferenceTargetKey(targetKey);
    setReferenceActionError(null);
    try {
      await onBindDiscoveredReference(image, target);
      setReferenceActionError(null);
    } catch (error) {
      setReferenceActionError(
        error instanceof Error ? error.message : t("comic.prompt.bindFailed"),
      );
    } finally {
      setAddingReferenceTargetKey(null);
    }
  };
  const unbindDiscoveredReference = async (
    assetKind: MangaReferenceAssetKind,
    assetId: number,
    targetKey: string,
  ) => {
    if (!onUnbindDiscoveredReference || addingReferenceTargetKey != null) return;
    setAddingReferenceTargetKey(targetKey);
    setReferenceActionError(null);
    try {
      await onUnbindDiscoveredReference(assetKind, assetId);
      setReferenceActionError(null);
    } catch (error) {
      setReferenceActionError(
        error instanceof Error ? error.message : t("comic.prompt.bindFailed"),
      );
    } finally {
      setAddingReferenceTargetKey(null);
    }
  };
  const generateDiscoveredReference = async (target: ReferenceTarget, targetKey: string) => {
    if (!onGenerateDiscoveredReference || generatingReferenceTargetKey != null) return;
    setGeneratingReferenceTargetKey(targetKey);
    setReferenceActionError(null);
    try {
      await onGenerateDiscoveredReference(page.prompt.id, target);
    } catch (error) {
      setReferenceActionError(error instanceof Error ? error.message : t("comic.prompt.referenceGenerationFailed"));
    } finally {
      setGeneratingReferenceTargetKey(null);
    }
  };
  if (
    referenceTargets.length === 0 &&
    page.referenceBindings.length === 0 &&
    unavailableSystemReferences.length === 0
  ) {
    return (
      <div className="border-l-2 border-book-border/70 pl-2 text-xs text-book-text-muted">
        {page.referenceDiscovery == null
          ? t("comic.prompt.referenceDiscoveryPending")
          : t("comic.prompt.noReferenceNeeds")}
      </div>
    );
  }

  return (
    <div className="divide-y divide-book-border/35">
      {page.referenceDiscovery == null ? (
        <p role="status" className="py-2 text-caption text-book-text-muted">
          {t("comic.prompt.referenceDiscoveryPending")}
        </p>
      ) : referenceTargets.length === 0 ? (
        <p role="status" className="py-2 text-caption text-book-text-muted">
          {t("comic.prompt.noReferenceNeeds")}
        </p>
      ) : null}
      {referenceTargets.length > 0 ? (
        <section className="space-y-2 border-y border-book-border/35 py-3">
          <h4 className="text-xs font-medium text-book-text-main">{t("comic.prompt.referenceLibrary")}</h4>
          {assetsLoading ? (
            <p role="status" className="text-caption text-book-text-muted">
              {t("comic.prompt.loadingImageAssets")}
            </p>
          ) : null}
          {referenceActionError ? (
            <div role="alert" className="flex items-center gap-2 text-caption text-error">
              <span className="min-w-0 flex-1">{referenceActionError}</span>
            </div>
          ) : null}
          {referenceTargets.map((target, targetIndex) => {
            const targetKey = referenceTargetKey(target, targetIndex);
            const bindingPickerOpen = bindingReferenceTargetKey === targetKey;
            const bindingPickerId = `${searchHintPrefix}-reference-picker-${targetIndex}`;
            const imageGroups = referenceImagesByTarget.get(targetKey);
            const targetReferences = discoveredReferences.filter(
              (reference) =>
                reference.target_kind === target.targetKind &&
                reference.target_name === target.targetName,
            );
            const referenceLimitMessage = targetReferences.length >= MANGA_REFERENCE_IMAGES_PER_TARGET
              ? t("comic.prompt.referenceTargetLimitReached")
              : discoveredReferences.length >= MANGA_PAGE_REFERENCE_IMAGE_LIMIT
                ? t("comic.prompt.referencePageLimitReached", { limit: MANGA_PAGE_REFERENCE_IMAGE_LIMIT })
                : null;
            const referenceMutationDisabled = chapterLocked || assetsLoading || addingReferenceTargetKey != null || generatingReferenceTargetKey != null;
            const referenceAdditionDisabled = referenceMutationDisabled || referenceLimitMessage != null;
            const systemPortrait = target.targetKind === "character"
              ? systemPortraitBindings.find((binding) =>
                  binding.characterName === target.targetName,
                )?.systemAsset
              : undefined;
            const targetImages = imageGroups == null
              ? []
              : [...new Map([
                  ...targetReferences.flatMap((reference) => {
                    const image = reference.asset.asset_kind === "generated_image"
                      ? referenceImageById.get(reference.asset.asset_id)
                      : undefined;
                    return image ? [[image.id, image] as const] : [];
                  }),
                  ...(bindingPickerOpen
                    ? imageGroups.all
                    : imageGroups.matches).map((image) => [image.id, image] as const),
                ])].map(([, image]) => image);
            const visibleImageCount = visibleImageCounts[targetKey] ?? REFERENCE_IMAGE_PAGE_SIZE;
            return (
              <article
                key={targetKey}
                className="space-y-2 rounded-md border border-book-border/40 bg-book-bg-paper/35 p-2"
              >
                <div className="flex min-w-0 flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                  <div className="min-w-0 flex-1">
                    <h5 className="truncate text-xs font-semibold text-book-text-main">
                      {referenceTargetKindLabel(target.targetKind)} · {target.targetName}
                    </h5>
                    {target.description ? (
                      <p className="mt-1 text-caption text-book-text-sub">
                        {target.description}
                      </p>
                    ) : null}
                    {target.usage ? (
                      <p className="text-caption text-book-text-muted">
                        {target.usage}
                      </p>
                    ) : null}
                  </div>
                  <div className="flex shrink-0 flex-wrap items-center gap-2">
                    {canEditReferenceDiscovery ? (
                      <BookButton
                        size="sm"
                        variant={bindingPickerOpen ? "primary" : "secondary"}
                        className="inline-flex items-center justify-center gap-1.5"
                        aria-label={t("comic.prompt.bindReferenceFor", { target: target.targetName })}
                        aria-expanded={bindingPickerOpen}
                        aria-controls={bindingPickerId}
                        title={referenceLimitMessage ?? t("comic.prompt.bindReferenceHint")}
                        disabled={referenceMutationDisabled || (!bindingPickerOpen && referenceLimitMessage != null)}
                        onClick={() => {
                          setBindingReferenceTargetKey(bindingPickerOpen ? null : targetKey);
                          setReferenceImageQuery("");
                        }}
                      >
                        <Link2 size={14} aria-hidden="true" />
                        {t("comic.prompt.bindReference")}
                      </BookButton>
                    ) : null}
                    {onGenerateDiscoveredReference && referenceRequirements.some((requirement) =>
                      requirement.target_kind === target.targetKind && requirement.target_name === target.targetName,
                    ) ? (
                      <BookButton
                        size="sm"
                        variant="secondary"
                        className="inline-flex shrink-0 items-center justify-center gap-1.5"
                        aria-label={t("comic.prompt.generateReferenceFor", { target: target.targetName })}
                        title={referenceLimitMessage ?? t("comic.prompt.generateReferenceHint")}
                        disabled={referenceAdditionDisabled}
                        onClick={() => void generateDiscoveredReference(target, targetKey)}
                      >
                        {generatingReferenceTargetKey === targetKey
                          ? <LoaderCircle size={14} className="animate-spin motion-reduce:animate-none" aria-hidden="true" />
                          : <ImagePlus size={14} aria-hidden="true" />}
                        {generatingReferenceTargetKey === targetKey ? t("comic.prompt.generatingReference") : t("comic.prompt.generateReference")}
                      </BookButton>
                    ) : null}
                  </div>
                </div>
                <div id={bindingPickerId} hidden={!bindingPickerOpen} className="space-y-2 rounded-md border border-book-border/50 p-2">
                  <p className="text-caption text-book-text-sub">{t("comic.prompt.bindReferenceHint")}</p>
                  <div className="flex flex-wrap items-end gap-2">
                    <div className="min-w-0 flex-1 basis-48">
                      <BookInput
                        label={t("comic.prompt.searchReferenceMaterials")}
                        placeholder={t("comic.prompt.searchReferenceMaterialsPlaceholder")}
                        value={referenceImageQuery}
                        onChange={(event) => setReferenceImageQuery(event.currentTarget.value)}
                        disabled={referenceMutationDisabled}
                      />
                    </div>
                    <label className={[
                      "relative inline-flex min-h-8 items-center justify-center gap-1 rounded-md border px-2 text-xs font-medium transition-colors focus-within:outline focus-within:outline-1 focus-within:outline-book-primary",
                      referenceAdditionDisabled
                        ? "cursor-not-allowed border-book-border/40 text-book-text-muted opacity-50"
                        : "cursor-pointer border-book-border/60 bg-book-bg-paper/70 text-book-text-sub hover:border-book-primary/60 hover:text-book-text-main",
                    ].join(" ")}>
                      {addingReferenceTargetKey === targetKey
                        ? <LoaderCircle size={14} className="animate-spin motion-reduce:animate-none" aria-hidden="true" />
                        : <Upload size={14} aria-hidden="true" />}
                      {addingReferenceTargetKey === targetKey ? t("comic.prompt.uploadingReference") : t("comic.prompt.uploadAndBindReference")}
                      <input
                        type="file"
                        accept={supportedImageAccept()}
                        aria-label={t("comic.prompt.uploadAndBindReferenceFor", { target: target.targetName })}
                        disabled={referenceAdditionDisabled}
                        className="sr-only"
                        onChange={(event) => {
                          const file = event.currentTarget.files?.[0];
                          event.currentTarget.value = "";
                          if (file) void bindDiscoveredReference(file, target, targetKey);
                        }}
                      />
                    </label>
                  </div>
                </div>
                {systemPortrait ? (
                  <div className="grid min-w-0 grid-cols-[40px_minmax(0,1fr)] items-center gap-2">
                    <div className="flex h-10 w-10 items-center justify-center overflow-hidden rounded-md border border-book-border/50 bg-book-bg-tertiary text-caption text-book-text-muted">
                      <ZoomableImage
                        src={toMediaSrc(systemPortrait.image_path) ?? systemPortrait.image_path}
                        alt={target.targetName}
                        className="h-full w-full object-cover"
                      />
                    </div>
                    <span className="min-w-0 text-caption text-book-text-muted">
                      {t("comic.prompt.systemAutoSelected")}
                      <span className="ml-2">
                        {t("comic.prompt.referenceNumber", {
                          number: systemPortraitBindings.findIndex((binding) =>
                            binding.characterName === target.targetName,
                          ) + 1,
                        })}
                      </span>
                    </span>
                  </div>
                ) : null}
                {targetReferences.length > 0 ? (
                  <p className="text-caption text-success">
                    {t("comic.prompt.referenceTargetBoundCount", { count: targetReferences.length })}
                  </p>
                ) : null}
                {!assetsLoading && targetImages.length === 0 ? (
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <p className="text-caption text-book-text-muted">
                      {referenceImages.length === 0
                        ? t("comic.prompt.noReferenceImagesInLibrary")
                        : referenceImageQuery.trim()
                          ? t("comic.prompt.noMatchingReferenceCandidates")
                          : t("comic.prompt.noReferenceImageMatches")}
                    </p>
                    {referenceImages.length > 0 &&
                    !referenceImageQuery.trim() &&
                    !bindingPickerOpen ? (
                      <BookButton
                        size="sm"
                        variant="ghost"
                        disabled={chapterLocked || addingReferenceTargetKey != null}
                        onClick={() => setBindingReferenceTargetKey(targetKey)}
                      >
                        {t("comic.prompt.showAllReferenceImages")}
                      </BookButton>
                    ) : null}
                  </div>
                ) : !assetsLoading ? (
                  <>
                    <div className="grid grid-cols-2 gap-2 md:grid-cols-3">
                      {targetImages.slice(0, visibleImageCount).map((image) => {
                        const imageSrc = toMediaSrc(image.file_path);
                        const existingBinding = discoveredReferences.find(
                          (reference) => reference.asset.asset_kind === "generated_image" &&
                            reference.asset.asset_id === image.id,
                        );
                        const alreadyBound = existingBinding != null;
                        const boundToThisTarget = existingBinding?.target_kind === target.targetKind &&
                          existingBinding.target_name === target.targetName;
                        const boundTarget = existingBinding
                          ? `${referenceTargetKindLabel(existingBinding.target_kind)} · ${existingBinding.target_name}`
                          : null;
                        const alsoBoundAsUserReference = target.targetKind === "character" &&
                          page.referenceBindings.some((binding) =>
                            binding.characterName === target.targetName &&
                            binding.userAddedAssets.some((asset) =>
                              asset.asset_kind === "generated_image" && asset.asset_id === image.id,
                            ),
                          );
                        const systemReferenceIndex = discoveredReferences.findIndex(
                          (reference) =>
                            reference.asset.asset_kind === "generated_image" &&
                            reference.asset.asset_id === image.id,
                        );
                        const imageLabel =
                          image.prompt.trim() ||
                          t("comic.prompt.projectImage");
                        return (
                          <div
                            key={image.id}
                            className="min-w-0 overflow-hidden rounded-md border border-book-border/40 bg-book-bg-paper/60"
                          >
                            <div className="aspect-[4/3] overflow-hidden bg-book-bg-tertiary">
                              {imageSrc ? (
                                <ZoomableImage
                                  src={imageSrc}
                                  alt={imageLabel}
                                  className="h-full w-full object-cover"
                                  loading="lazy"
                                  decoding="async"
                                />
                              ) : (
                                <div className="flex h-full items-center justify-center text-book-text-muted">
                                  <Image size={16} />
                                </div>
                              )}
                            </div>
                            <div className="space-y-1 p-1.5">
                              <p className="truncate text-caption text-book-text-main" title={imageLabel}>
                                {imageLabel}
                              </p>
                              {alsoBoundAsUserReference ? (
                                <p className="text-caption text-book-text-muted">
                                  {t("comic.prompt.alsoBoundAsUserReference", {
                                    character: target.targetName,
                                  })}
                                </p>
                              ) : null}
                              {boundToThisTarget && systemReferenceIndex >= 0 ? (
                                <p className="text-caption text-book-text-muted">
                                  {t("comic.prompt.referenceNumber", {
                                    number: portraitReferenceCount + systemReferenceIndex + 1,
                                  })}
                                </p>
                              ) : null}
                              <BookButton
                                size="sm"
                                variant={boundToThisTarget ? "ghost" : alreadyBound ? "primary" : "secondary"}
                                className="w-full"
                                aria-label={boundToThisTarget
                                  ? t("comic.prompt.removeReferenceFromTarget", { target: target.targetName })
                                  : t("comic.prompt.bindReferenceImageTo", {
                                      image: imageLabel,
                                      target: target.targetName,
                                    })}
                                disabled={
                                  referenceMutationDisabled ||
                                  (boundToThisTarget
                                    ? !onUnbindDiscoveredReference
                                    : alreadyBound || referenceLimitMessage != null || !canEditReferenceDiscovery)
                                }
                                onClick={() => {
                                  if (boundToThisTarget && existingBinding) {
                                    void unbindDiscoveredReference(
                                      existingBinding.asset.asset_kind,
                                      existingBinding.asset.asset_id,
                                      targetKey,
                                    );
                                  } else if (!alreadyBound) {
                                    void bindDiscoveredReference(image, target, targetKey);
                                  }
                                }}
                              >
                                {addingReferenceTargetKey === targetKey ? (
                                  <LoaderCircle size={12} className="animate-spin" />
                                ) : boundToThisTarget ? (
                                  <X size={12} />
                                ) : (
                                  <Link2 size={12} />
                                )}
                                {boundToThisTarget
                                  ? t("comic.prompt.removeReference")
                                  : alreadyBound
                                    ? t("comic.prompt.referenceAlreadyBoundTo", { target: boundTarget ?? "" })
                                    : referenceLimitMessage ?? t("comic.prompt.bindDiscoveredReference")}
                              </BookButton>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <p role="status" aria-live="polite" className="text-caption text-book-text-muted">
                        {targetImages.length > visibleImageCount
                          ? t("comic.prompt.referenceCandidateTruncated", {
                              total: targetImages.length,
                              shown: Math.min(visibleImageCount, targetImages.length),
                            })
                          : t("comic.prompt.referenceCandidateCount", { total: targetImages.length })}
                      </p>
                      {targetImages.length > visibleImageCount ? (
                        <BookButton
                          size="sm"
                          variant="ghost"
                          disabled={chapterLocked || addingReferenceTargetKey != null}
                          onClick={() => setVisibleImageCounts((current) => ({
                            ...current,
                            [targetKey]: visibleImageCount + REFERENCE_IMAGE_PAGE_SIZE,
                          }))}
                        >
                          {t("comic.prompt.showMoreReferenceImages")}
                        </BookButton>
                      ) : null}
                      {!bindingPickerOpen &&
                      !referenceImageQuery.trim() &&
                      imageGroups?.all.length !== imageGroups?.matches.length ? (
                        <BookButton
                          size="sm"
                          variant="ghost"
                          disabled={chapterLocked || addingReferenceTargetKey != null}
                          onClick={() => setBindingReferenceTargetKey(targetKey)}
                        >
                          {t("comic.prompt.showAllReferenceImages")}
                        </BookButton>
                      ) : null}
                    </div>
                  </>
                ) : null}
              </article>
            );
          })}
        </section>
      ) : null}
      {unavailableSystemReferences.length > 0 ? (
        <section className="space-y-2 border-y border-book-border/35 py-3">
          <h4 className="text-xs font-medium text-book-text-main">
            {t("comic.prompt.discoveredReferences")}
          </h4>
          <div className="divide-y divide-book-border/30">
            {unavailableSystemReferences.map((reference) => {
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
                  className="grid min-w-0 grid-cols-[40px_minmax(0,1fr)_auto] items-center gap-2 py-2"
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
                      {reference.usage}
                    </p>
                    <span className="text-caption text-book-text-muted">
                      {t("comic.prompt.referenceNumber", {
                        number: portraitReferenceCount + discoveredReferences.indexOf(reference) + 1,
                      })}
                    </span>
                  </div>
                  <BookButton
                    size="sm"
                    variant="ghost"
                    aria-label={t("comic.prompt.removeReferenceFromTarget", {
                      target: reference.target_name,
                    })}
                    disabled={chapterLocked || addingReferenceTargetKey != null || !onUnbindDiscoveredReference}
                    onClick={() => void unbindDiscoveredReference(
                      reference.asset.asset_kind,
                      reference.asset.asset_id,
                      `unavailable:${reference.asset.asset_id}`,
                    )}
                  >
                    <X size={12} /> {t("comic.prompt.removeReference")}
                  </BookButton>
                </div>
              );
            })}
          </div>
        </section>
      ) : null}
      {page.referenceBindings.map((binding, bindingIndex) => {
        const candidateSearch = candidatesByCharacter.get(binding.characterName)!;
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
          chapterLocked ||
          assetsLoading ||
          uploadingReferenceBindingKey != null;
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
              {binding.userAddedAssets.length > 0 ? (
                <span className="text-caption text-warning">
                  {t("comic.prompt.userReferenceOnly")}
                </span>
              ) : null}
            </div>

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
                        {[
                          candidate.label,
                          candidate.subLabel,
                          useSearchPicker ? candidateIdentityLabel(candidate) : null,
                          useSearchPicker ? candidate.searchPreview : null,
                        ].filter(Boolean).join(" · ")}
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
                          {[
                            selectedUser.label,
                            candidateIdentityLabel(selectedUser),
                          ].filter(Boolean).join(" · ")}
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
                    const sourceImage = selectedUser.assetKind === "generated_image"
                      ? referenceImageById.get(selectedUser.assetId)
                      : undefined;
                    void onBindReferenceImage({
                      action: "add_user_image",
                      prompt_id: page.prompt.id,
                      character_name: binding.characterName,
                      asset_kind: selectedUser.assetKind,
                      asset_id: selectedUser.assetId,
                    }, sourceImage);
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
