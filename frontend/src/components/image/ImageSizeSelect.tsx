import { ThemedSelect } from "../ui/ThemedSelect";
import { useEffect, useState } from "react";
import {
  listImageConfigs,
  normalizeImageSizes,
  type ImageSize,
} from "../../api/image";
import { useI18n } from "../../i18n";

export interface ImageGenerationSizeState {
  error: Error | null;
  loading: boolean;
  size: ImageSize | null;
  sizes: ImageSize[];
  setSize: (size: ImageSize) => void;
}

export function useImageGenerationSize(): ImageGenerationSizeState {
  const [sizes, setSizes] = useState<ImageSize[]>([]);
  const [size, setSize] = useState<ImageSize | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;
    setLoading(true);
    listImageConfigs()
      .then((configs) => {
        const config = configs.find((item) => item.is_default);
        if (!config) throw new Error("未找到当前图片配置");
        const supportedSizes = normalizeImageSizes(
          config.supported_sizes,
          `图片配置 ${config.name}`,
        );
        if (!active) return;
        setSizes(supportedSizes);
        setSize((current) =>
          current && supportedSizes.some(
            (candidate) =>
              candidate.width === current.width && candidate.height === current.height,
          )
            ? current
            : supportedSizes[0],
        );
        setError(null);
      })
      .catch((reason: unknown) => {
        if (!active) return;
        setSizes([]);
        setSize(null);
        setError(reason instanceof Error ? reason : new Error(String(reason)));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  return { error, loading, size, sizes, setSize };
}

interface ImageSizeSelectProps {
  disabled?: boolean;
  error?: Error | null;
  label?: string;
  loading?: boolean;
  onChange: (size: ImageSize) => void;
  sizes: readonly ImageSize[];
  value: ImageSize | null;
}

export function ImageSizeSelect({
  disabled = false,
  error = null,
  label,
  loading = false,
  onChange,
  sizes,
  value,
}: ImageSizeSelectProps) {
  const { t } = useI18n();
  const selectedValue = value ? `${value.width}x${value.height}` : "";
  return (
    <label className="block min-w-0 text-caption text-book-text-muted">
      {label ?? t("settingsForm.supportedSizes")}
      <ThemedSelect
        value={selectedValue}
        disabled={disabled || loading || error != null || sizes.length === 0}
        onChange={(event) => {
          const next = sizes.find(
            (candidate) => `${candidate.width}x${candidate.height}` === event.target.value,
          );
          if (next) onChange(next);
        }}
        className="pyqt-input mt-1 w-full min-w-0 text-xs"
      >
        <option value="" disabled>
          {loading
            ? t("settingsForm.imageSizeLoading")
            : error || sizes.length === 0
              ? t("settingsForm.imageSizeUnavailable")
              : t("settingsForm.selectImageSize")}
        </option>
        {sizes.map((candidate) => (
          <option
            key={`${candidate.width}x${candidate.height}`}
            value={`${candidate.width}x${candidate.height}`}
          >
            {candidate.width} x {candidate.height}
          </option>
        ))}
      </ThemedSelect>
      {error ? (
        <span role="alert" className="mt-1 block text-small text-warning">
          {t("settingsForm.imageSizeUnavailable")}
        </span>
      ) : null}
    </label>
  );
}
