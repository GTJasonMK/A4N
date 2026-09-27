import { Music, X } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { ZoomableImage } from "../../../components/ui/ImageLightbox";
import { MediaAssetIdBinder } from "./MediaAssetIdBinder";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { useI18n } from "../../../i18n";
import type { GalgameMediaBindingSummary } from "../../../api/galgame";

export const LineMediaBindings: React.FC<{
  bindings: GalgameMediaBindingSummary[];
  disabled: boolean;
  onImportAudio: (role: "bgm" | "sfx") => void;
  onBindMedia: (role: "cg" | "bgm" | "sfx", mediaAssetId: number) => void;
  onUnbindMedia: (binding: GalgameMediaBindingSummary) => void;
}> = ({ bindings, disabled, onImportAudio, onBindMedia, onUnbindMedia }) => {
  const { t } = useI18n();
  const audioBindings = bindings.filter(
    (item) => item.binding.role === "bgm" || item.binding.role === "sfx",
  );
  const cgBindings = bindings.filter((item) => item.binding.role === "cg");
  const bindingCount = audioBindings.length + cgBindings.length;
  return (
    <details className="mt-3 border-t border-book-border/40 pt-2.5">
      <summary className="cursor-pointer rounded-lg px-1 py-1 text-small font-medium text-book-text-main hover:bg-book-bg-tertiary/55">
        {t("galgame.mediaAndAudio")} ·{" "}
        {bindingCount > 0
          ? t("galgame.bindingCount", { count: bindingCount })
          : t("galgame.unbound")}
      </summary>
      <div className="mt-2.5">
        <ImageBindingStrip
          title="CG"
          emptyLabel=""
          bindings={cgBindings}
          disabled={disabled}
          bindControl={
            <MediaAssetIdBinder
              label={t("galgame.bindCg")}
              disabled={disabled}
              onBind={(mediaAssetId) => onBindMedia("cg", mediaAssetId)}
            />
          }
          onUnbindMedia={onUnbindMedia}
        />
        <div className="mb-2 divide-y divide-book-border/35 rounded-xl bg-book-bg-paper/45 lg:grid lg:grid-cols-2 lg:divide-x lg:divide-y-0">
          <AudioBindingControl
            label="BGM"
            disabled={disabled}
            onImport={() => onImportAudio("bgm")}
            onBind={(mediaAssetId) => onBindMedia("bgm", mediaAssetId)}
          />
          <AudioBindingControl
            label="SFX"
            disabled={disabled}
            onImport={() => onImportAudio("sfx")}
            onBind={(mediaAssetId) => onBindMedia("sfx", mediaAssetId)}
          />
        </div>
        {audioBindings.length ? (
          <div className="divide-y divide-book-border/35 rounded-xl bg-book-bg-paper/45 px-2.5">
            {audioBindings.map((item) => {
              const src = item.asset.file_path
                ? toMediaSrc(item.asset.file_path)
                : "";
              return (
                <div
                  key={item.binding.id}
                  className="grid gap-2 py-2.5 text-small text-book-text-muted md:grid-cols-[64px_minmax(0,1fr)_auto]"
                >
                  <span className="font-semibold text-book-text-main">
                    {item.binding.role === "bgm" ? "BGM" : "SFX"}
                  </span>
                  {src ? (
                    <audio src={src} controls className="h-8 w-full" />
                  ) : (
                    <span>{t("galgame.audioMissing")}</span>
                  )}
                  <BookButton
                    variant="secondary"
                    size="sm"
                    onClick={() => onUnbindMedia(item)}
                    disabled={disabled}
                    aria-label={t("galgame.unbindMedia", {
                      title: item.binding.role === "bgm" ? "BGM" : "SFX",
                    })}
                  >
                    <X size={13} />
                  </BookButton>
                </div>
              );
            })}
          </div>
        ) : null}
      </div>
    </details>
  );
};

export const ImageBindingStrip: React.FC<{
  title: string;
  emptyLabel: string;
  bindings: GalgameMediaBindingSummary[];
  disabled: boolean;
  bindControl?: React.ReactNode;
  onUnbindMedia: (binding: GalgameMediaBindingSummary) => void;
}> = ({
  title,
  emptyLabel,
  bindings,
  disabled,
  bindControl,
  onUnbindMedia,
}) => {
  const { t } = useI18n();
  if (!bindings.length) {
    return (
      <div className="mb-2 space-y-2">
        {emptyLabel ? (
          <div className="border border-dashed border-book-border bg-book-bg-paper px-3 py-4 text-center text-small text-book-text-muted">
            {emptyLabel}
          </div>
        ) : null}
        {bindControl}
      </div>
    );
  }
  return (
    <div className="mb-2 space-y-2">
      <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-1">
        {bindings.map((item) => {
          const mediaPath = item.asset.thumbnail_path || item.asset.file_path;
          const src = mediaPath ? toMediaSrc(mediaPath) : "";
          return (
            <div
              key={item.binding.id}
              className="overflow-hidden rounded-xl border border-book-border/45 bg-book-bg-paper"
            >
              <div className="flex items-center gap-2 px-2.5 py-2">
                <span className="min-w-0 flex-1 truncate text-small font-semibold text-book-text-main">
                  {title}
                </span>
                <button
                  type="button"
                  onClick={() => onUnbindMedia(item)}
                  disabled={disabled}
                  className="inline-flex h-7 w-7 items-center justify-center rounded-lg text-book-text-muted hover:bg-book-bg-tertiary hover:text-book-text-main disabled:opacity-45"
                  aria-label={t("galgame.unbindMedia", { title })}
                >
                  <X size={13} />
                </button>
              </div>
              <div className="aspect-video w-full overflow-hidden bg-book-bg">
                {src ? (
                  <ZoomableImage
                    src={src}
                    alt={title}
                    className="h-full w-full object-cover"
                  />
                ) : (
                  <div className="flex h-full items-center justify-center text-small text-book-text-muted">
                    {t("galgame.imageMissing")}
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
      {bindControl}
    </div>
  );
};

const AudioBindingControl: React.FC<{
  label: string;
  disabled: boolean;
  onImport: () => void;
  onBind: (mediaAssetId: number) => void;
}> = ({ label, disabled, onImport, onBind }) => {
  const { t } = useI18n();
  return (
    <section className="p-2.5">
      <div className="mb-2 text-small font-semibold text-book-text-main">
        {label}
      </div>
      <BookButton
        variant="secondary"
        size="sm"
        className="mb-2 w-full"
        onClick={onImport}
        disabled={disabled}
      >
        <Music size={13} />
        {t("galgame.importAudio", { label })}
      </BookButton>
      <MediaAssetIdBinder
        label={t("galgame.bindAudio", { label })}
        disabled={disabled}
        onBind={onBind}
      />
    </section>
  );
};
