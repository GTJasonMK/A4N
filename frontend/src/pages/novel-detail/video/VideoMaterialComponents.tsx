import React from "react";
import { BadgeCheck, Box, Clipboard, MapPinned, RefreshCw, Sparkles, Star, Upload } from "lucide-react";
import type { GeneratedImage } from "../../../api/image";
import type {
  ProfileMaterialFilter,
  ProfileMaterialGroup,
  ProfileMaterialKind,
  ProfileMaterialKindFilter,
  ProfileMaterialSort,
  ProfileMaterialState,
} from "../../../api/profileMaterials";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { supportedImageAccept } from "../../../utils/imageUpload";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { chapterRangeText } from "../shared";
import { useI18n, type MessageKey, type MessageTranslator } from "../../../i18n";
export const kindOptions = (t: MessageTranslator): Array<{ value: ProfileMaterialKindFilter; label: string }> => [
  { value: "all", label: t("video.material.all") },
  { value: "scene", label: t("video.material.scene") },
  { value: "object", label: t("video.material.object") },
];

export const filterOptions = (t: MessageTranslator): Array<{ value: ProfileMaterialFilter; label: string }> => [
  { value: "all", label: t("video.material.allStates") },
  { value: "missing", label: t("video.material.missing") },
  { value: "stale", label: t("video.material.stale") },
  { value: "unknown", label: t("video.material.old") },
  { value: "ready", label: t("video.material.ready") },
  { value: "favorite", label: t("video.material.favorite") },
];

export const sortOptions = (t: MessageTranslator): Array<{ value: ProfileMaterialSort; label: string }> => [
  { value: "missing_first", label: t("video.material.missingFirst") },
  { value: "stale_first", label: t("video.material.staleFirst") },
  { value: "newest", label: t("video.material.newest") },
  { value: "importance", label: t("video.material.importance") },
];

export const materialStateLabel = (value: ProfileMaterialState, t: MessageTranslator): string =>
  t(`video.material.state${value[0].toUpperCase()}${value.slice(1)}` as MessageKey);

export const materialKindLabel = (value: ProfileMaterialKind, t: MessageTranslator): string =>
  t(`video.material.${value}` as MessageKey);

const IMPORTANCE_KEYS: Record<string, MessageKey> = {
  primary: "video.material.importancePrimary",
  secondary: "video.material.importanceSecondary",
  minor: "video.material.importanceMinor",
  major: "video.material.importanceMajor",
  supporting: "video.material.importanceSupporting",
  background: "video.material.importanceBackground",
  high: "video.material.importanceHigh",
  medium: "video.material.importanceMedium",
  low: "video.material.importanceLow",
  normal: "video.material.importanceNormal",
  critical: "video.material.importanceCritical",
};

const CATEGORY_KEYS: Record<string, MessageKey> = {
  room: "video.material.categoryRoom",
  building: "video.material.categoryBuilding",
  street: "video.material.categoryStreet",
  city: "video.material.categoryCity",
  wilderness: "video.material.categoryWilderness",
  route: "video.material.categoryRoute",
  landmark: "video.material.categoryLandmark",
  location: "video.material.categoryLocation",
  interior: "video.material.categoryInterior",
  exterior: "video.material.categoryExterior",
  region: "video.material.categoryRegion",
  world: "video.material.categoryWorld",
  organization: "video.material.categoryOrganization",
  weapon: "video.material.categoryWeapon",
  prop: "video.material.categoryProp",
  tool: "video.material.categoryTool",
  token: "video.material.categoryToken",
  document: "video.material.categoryDocument",
  vehicle: "video.material.categoryVehicle",
  clothing: "video.material.categoryClothing",
  location_object: "video.material.categoryLocationObject",
  environment_object: "video.material.categoryEnvironmentObject",
  artifact: "video.material.categoryArtifact",
  consumable: "video.material.categoryConsumable",
  other: "video.material.categoryOther",
};

export const categoryLabel = (value: string, t: MessageTranslator): string => {
  const key = CATEGORY_KEYS[value];
  return key ? t(key) : t("video.material.uncategorized");
};

export const importanceLabel = (value: string, t: MessageTranslator): string => {
  const key = IMPORTANCE_KEYS[value];
  return key ? t(key) : t("video.material.unmarked");
};

export const groupKey = (group: ProfileMaterialGroup): string => `${group.kind}:${group.profile.id}`;
export function MaterialCard({
  group,
  busy,
  disabled,
  onCopyPrompt,
  onUpload,
  onGenerate,
  onOpenImage,
  onSetPrimary,
}: {
  group: ProfileMaterialGroup;
  busy: boolean;
  disabled: boolean;
  onCopyPrompt: () => void;
  onUpload: () => void;
  onGenerate: () => void;
  onOpenImage: (image: GeneratedImage) => void;
  onSetPrimary: (imageId: number | null) => void;
}) {
  const { t } = useI18n();
  const icon = group.kind === "scene" ? <MapPinned size={14} /> : <Box size={14} />;
  const hero = group.primary_image ?? group.images[0] ?? null;
  const heroSrc = hero ? toMediaSrc(hero.image.file_path) : null;
  const candidateImages = hero
    ? group.images.filter((item) => item.image.id !== hero.image.id)
    : group.images;
  const stateTone =
    group.material_state === "ready"
      ? "text-success"
      : group.material_state === "missing" || group.material_state === "stale"
        ? "text-warning"
        : "text-book-text-muted";
  const missingMaterial = group.material_state === "missing" || group.image_count === 0;
  const staleMaterial = group.material_state === "stale" && group.image_count > 0;
  const directGenerate = missingMaterial || staleMaterial;
  const menuItems: React.ComponentProps<typeof Dropdown>["items"] = [
    ...(!directGenerate
      ? [
          {
            label: t("video.material.regenerate"),
            icon: <Sparkles size={13} />,
            disabled,
            onClick: onGenerate,
          },
          { type: "divider" as const },
        ]
      : []),
    {
      label: t("video.material.copyPrompt"),
      icon: <Clipboard size={13} />,
      disabled,
      onClick: onCopyPrompt,
    },
    {
      label: t("video.material.upload"),
      icon: <Upload size={13} />,
      disabled,
      onClick: onUpload,
    },
    ...(group.primary_image_id != null
      ? [
          { type: "divider" as const },
          {
            label: t("video.material.clearPrimary"),
            disabled,
            onClick: () => onSetPrimary(null),
          },
        ]
      : []),
  ];
  return (
    <article
      className="a4n-flat-surface flex min-w-0 flex-col p-3"
      data-state={group.material_state}
    >
      <div className="mb-3 flex min-w-0 items-start gap-2.5">
        <span className="mt-0.5 shrink-0 text-book-primary">{icon}</span>
        <div className="min-w-0 flex-1">
          <h3 className="truncate text-sm font-medium text-book-text-main">
            {group.profile.name}
          </h3>
          <p className="mt-1 truncate text-caption text-book-text-muted">
            {materialKindLabel(group.kind, t)} · {categoryLabel(group.profile.category, t)} · {importanceLabel(group.profile.importance, t)} · {chapterRangeText(group.profile.first_seen_chapter, group.profile.last_seen_chapter, t)}
          </p>
        </div>
        <div className="shrink-0 text-right text-caption leading-relaxed">
          <div className={stateTone}>{materialStateLabel(group.material_state, t)}</div>
          <div className="text-book-text-muted">{t("video.material.imageCount", { count: group.image_count })}</div>
        </div>
      </div>

      <button
        type="button"
        disabled={!hero}
        onClick={() => hero && onOpenImage(hero.image)}
        className="relative flex aspect-[16/9] w-full items-center justify-center overflow-hidden rounded-2xl border border-book-border/45 bg-book-bg-tertiary text-xs text-book-text-muted disabled:cursor-default"
        title={hero ? t("video.material.openImage") : t("video.material.noImage")}
      >
        {heroSrc ? (
          <img
            src={heroSrc}
            alt={`${group.profile.name} ${t("video.material.primaryMaterial")}`}
            className="h-full w-full object-cover"
          />
        ) : (
          t("video.material.noImage")
        )}
        {hero ? (
          <div className="absolute left-2 top-2 flex gap-1.5 text-white">
            {hero.is_primary ? (
              <span
                className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-white/30 bg-black/35 backdrop-blur-md"
                title={t("video.material.primaryMaterial")}
                aria-label={t("video.material.primaryMaterial")}
              >
                <BadgeCheck size={13} />
              </span>
            ) : null}
            {hero.image.is_favorite ? (
              <span
                className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-white/30 bg-black/35 backdrop-blur-md"
                title={t("video.material.favoritedLabel")}
                aria-label={t("video.material.favoritedLabel")}
              >
                <Star size={13} fill="currentColor" />
              </span>
            ) : null}
          </div>
        ) : null}
      </button>

      <p className="mt-2 line-clamp-2 text-xs leading-relaxed text-book-text-sub">
        {group.profile.primary_text || t("video.material.noVisualDescription")}
      </p>
      <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-muted">
        {group.profile.secondary_text || t("video.material.noSupplementalInfo")}
      </p>

      <div className="mt-2 flex min-h-14 gap-2 overflow-x-auto pb-1 custom-scrollbar">
        {candidateImages.length > 0 ? (
          candidateImages.slice(0, 12).map((item) => {
            const src = toMediaSrc(item.image.file_path);
            return (
              <div key={item.image.id} className="group/thumb relative h-14 w-14 shrink-0">
                <button
                  type="button"
                  onClick={() => onOpenImage(item.image)}
                  className={`relative flex h-full w-full items-center justify-center overflow-hidden rounded-xl border bg-book-bg-tertiary text-caption text-book-text-muted ${
                    item.is_primary
                      ? "border-book-primary/80"
                      : "border-book-border/45"
                  }`}
                  title={t("video.material.materialImage")}
                >
                  {src ? (
                    <img
                      src={src}
                      alt={`${group.profile.name} ${t("video.material.candidateImage")}`}
                      className="h-full w-full object-cover"
                    />
                  ) : (
                    t("video.material.noImageShort")
                  )}
                  {item.staleness === "stale" ? (
                    <span
                      className="absolute right-1 top-1 h-2 w-2 rounded-full bg-warning ring-2 ring-black/25"
                      title={t("video.material.staleImage")}
                      aria-label={t("video.material.staleImage")}
                    />
                  ) : null}
                  {item.is_primary ? (
                    <BadgeCheck
                      size={12}
                      className="absolute bottom-0.5 left-0.5 text-book-primary"
                      fill="currentColor"
                    />
                  ) : null}
                </button>
                {!item.is_primary ? (
                  <button
                    type="button"
                    onClick={() => onSetPrimary(item.image.id)}
                    disabled={disabled}
                    className="absolute bottom-1 right-1 inline-flex h-6 w-6 items-center justify-center rounded-full border border-white/25 bg-black/40 text-white opacity-80 backdrop-blur-sm transition-opacity sm:opacity-0 sm:group-hover/thumb:opacity-100 sm:focus-visible:opacity-100 disabled:opacity-40"
                    title={t("video.material.setPrimary")}
                    aria-label={t("video.material.setPrimary")}
                  >
                    <BadgeCheck size={12} />
                  </button>
                ) : null}
              </div>
            );
          })
        ) : hero ? (
          <div className="flex h-14 w-full items-center justify-center rounded-xl border border-dashed border-book-border/50 bg-book-bg-paper/30 text-small text-book-text-muted">
            {t("video.material.noOtherCandidates")}
          </div>
        ) : (
          <div className="flex h-14 w-full items-center justify-center rounded-xl border border-dashed border-book-border/50 bg-book-bg-paper/30 text-small text-book-text-muted">
            {t("video.material.missingImage")}
          </div>
        )}
      </div>

      <div className="mt-auto flex items-center gap-2 pt-3">
        {busy ? (
          <span
            role="status"
            className="inline-flex min-h-9 min-w-0 flex-1 items-center justify-center gap-1.5 text-xs text-book-text-muted"
          >
            <LoadingSpinner size="sm" /> {t("video.material.generating")}
          </span>
        ) : directGenerate ? (
          <button
            type="button"
            onClick={onGenerate}
            disabled={disabled}
            className={`${missingMaterial ? "pyqt-btn-primary" : "pyqt-btn-secondary"} inline-flex min-h-9 min-w-0 flex-1 items-center justify-center gap-1.5 px-3 text-xs`}
            title={missingMaterial ? t("video.material.generateReusableTitle") : t("video.material.updateStaleTitle")}
          >
            {staleMaterial ? <RefreshCw size={13} /> : <Sparkles size={13} />}
            {staleMaterial ? t("video.material.updateImage") : t("video.material.generateImage")}
          </button>
        ) : (
          <span className="min-w-0 flex-1" aria-hidden="true" />
        )}
        <Dropdown items={menuItems} />
      </div>
    </article>
  );
}

export function ToolbarSelect({
  label,
  value,
  options,
  disabled,
  onChange,
  wide = false,
}: {
  label: string;
  value: string;
  options: Array<{ value: string; label: string }>;
  disabled?: boolean;
  onChange: (value: string) => void;
  wide?: boolean;
}) {
  return (
    <label
      className={`flex min-h-9 min-w-0 items-center gap-2 rounded-xl border border-book-border/60 bg-book-bg-paper/60 px-3 text-xs transition-colors focus-within:border-book-primary/70 focus-within:bg-book-bg-paper ${
        wide ? "w-full sm:col-span-2 sm:flex-1 sm:min-w-[210px]" : "sm:min-w-[130px]"
      }`}
    >
      <span className="shrink-0 text-book-text-muted">{label}</span>
      <ThemedSelect
        value={value}
        onChange={onChange}
        disabled={disabled}
        className="pyqt-input a4n-compact-select h-7 !min-h-7 min-w-0 flex-1 !border-transparent !bg-transparent !pl-1 text-xs text-book-text-main"
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </ThemedSelect>
    </label>
  );
}

export function ProfileMaterialUploadDialog({
  group,
  busy,
  setPrimary,
  onSetPrimaryChange,
  onClose,
  onCopyPrompt,
  onUpload,
}: {
  group: ProfileMaterialGroup | null;
  busy: boolean;
  setPrimary: boolean;
  onSetPrimaryChange: (value: boolean) => void;
  onClose: () => void;
  onCopyPrompt: () => void;
  onUpload: (file: File) => void;
}) {
  if (!group) return null;
  const { t } = useI18n();
  return (
    <Modal
      isOpen
      onClose={() => {
        if (!busy) onClose();
      }}
      title={t("video.material.uploadTitle")}
      subtitle={`${materialKindLabel(group.kind, t)} · ${group.profile.name}`}
      icon={<Upload size={15} />}
      maxWidthClassName="max-w-md"
      closeOnBackdrop={!busy}
      closeOnEscape={!busy}
    >
      <div className="space-y-4">
        <button
          type="button"
          role="switch"
          aria-checked={setPrimary}
          disabled={busy}
          onClick={() => onSetPrimaryChange(!setPrimary)}
          className="flex w-full items-center justify-between gap-3 border-b border-book-border/35 pb-3 text-left disabled:cursor-not-allowed disabled:opacity-60"
        >
          <span className="min-w-0">
            <span className="block text-xs font-medium text-book-text-main">
              {t("video.material.uploadAfterPrimary")}
            </span>
            <span className="mt-0.5 block text-small leading-relaxed text-book-text-muted">
              {t("video.material.uploadAfterPrimaryHelp")}
            </span>
          </span>
          <span
            className={`relative inline-flex h-6 w-11 shrink-0 rounded-full transition-colors ${
              setPrimary ? "bg-book-primary" : "bg-book-text-muted/25"
            }`}
            aria-hidden="true"
          >
            <span
              className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition-transform ${
                setPrimary ? "translate-x-[22px]" : "translate-x-0.5"
              }`}
            />
          </span>
        </button>

        <div className="rounded-2xl border border-dashed border-book-border/70 bg-book-bg-tertiary/35 p-4">
          <label
            className={`inline-flex min-h-10 items-center gap-2 rounded-xl bg-book-primary px-3.5 text-xs font-medium text-white transition-colors hover:bg-book-primary/90 ${
              busy ? "pointer-events-none opacity-60" : "cursor-pointer"
            }`}
          >
            <Upload size={14} />
            {t("video.material.chooseImage")}
            <input
              type="file"
              accept={supportedImageAccept()}
              disabled={busy}
              className="sr-only"
              onChange={(event) => {
                const file = event.target.files?.[0] ?? null;
                event.currentTarget.value = "";
                if (file) onUpload(file);
              }}
            />
          </label>
          <div className="mt-2 text-small text-book-text-muted">
            {t("video.material.supportedFormats")}
          </div>
        </div>

        <div className="flex min-w-0 items-center justify-between gap-3">
          <button
            type="button"
            onClick={onCopyPrompt}
            disabled={busy}
            className="inline-flex min-h-9 items-center gap-1.5 text-xs text-book-text-sub transition-colors hover:text-book-text-main disabled:opacity-50"
          >
            <Clipboard size={13} />
            {t("video.material.copyCurrentPrompt")}
          </button>
          {busy ? (
            <span role="status" className="inline-flex items-center gap-1.5 text-xs text-book-text-muted">
              <LoadingSpinner size="sm" />
              {t("video.material.uploading")}
            </span>
          ) : null}
        </div>
      </div>
    </Modal>
  );
}
